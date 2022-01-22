use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info};

use ate::prelude::*;

use crate::api::*;
use crate::error::*;
use crate::model::*;
use crate::opt::*;

use super::*;

pub async fn session_with_permissions<A>(
    purpose: &dyn OptsPurpose<A>,
    token_path: &str,
    auth_url: &url::Url,
    sudo: bool,
) -> Result<(A, AteSessionType), AteError>
where
    A: Clone,
{
    let session: AteSessionType;
    if let Purpose::<A>::Domain {
        domain_name: group_name,
        wallet_name: _,
        action: _,
    } = purpose.purpose()
    {
        session = main_session_group(
            None,
            Some(token_path.to_string()),
            group_name,
            sudo,
            None,
            Some(auth_url.clone()),
            "Domain name",
        )
        .await?
        .into();
    } else if sudo {
        session = main_session_sudo(
            None,
            Some(token_path.to_string()),
            None,
            Some(auth_url.clone()),
        )
        .await?
        .into();
    } else {
        session = main_session_user(None, Some(token_path.to_string()), Some(auth_url.clone()))
            .await?
            .into();
    }
    Ok((purpose.action(), session))
}

pub async fn get_identity<A>(
    purpose: &dyn OptsPurpose<A>,
    session: &dyn AteSession,
) -> Result<String, AteError>
where
    A: Clone,
{
    let identity = match purpose.purpose() {
        Purpose::Personal {
            wallet_name: _,
            action: _,
        } => session.user().identity().to_string(),
        Purpose::Domain {
            domain_name: group_name,
            wallet_name: _,
            action: _,
        } => group_name,
    };
    debug!("identity={}", identity);
    Ok(identity)
}

pub fn get_wallet_name<A>(purpose: &dyn OptsPurpose<A>) -> std::result::Result<String, AteError>
where
    A: Clone,
{
    // Validate a wallet name is supplied correctly
    let wallet_name = purpose.wallet_name().to_lowercase();
    if wallet_name.len() <= 0 {
        eprintln!("No wallet name was supplied.");
        std::process::exit(1);
    }
    debug!("wallet_name={}", wallet_name);

    Ok(wallet_name)
}

pub async fn create_wallet(
    dio: &Arc<DioMut>,
    auth: &url::Url,
    registry: &Arc<Registry>,
    identity: &String,
    wallet_name: &String,
    parent_key: &PrimaryKey,
    gst_country: Country,
) -> std::result::Result<DaoMut<Wallet>, AteError> {
    // Get the sudo rights from the session (as we will use these for the wallet)
    let session = dio.session();
    let sudo_read = {
        let sudo_read = match session.read_keys(AteSessionKeyCategory::SudoKeys).next() {
            Some(a) => a,
            None => {
                eprintln!("Login sudo rights do not have a read key.");
                std::process::exit(1);
            }
        };
        if session
            .write_keys(AteSessionKeyCategory::SudoKeys)
            .next()
            .is_none()
        {
            eprintln!("Login sudo rights do not have a write key.");
            std::process::exit(1);
        };
        sudo_read
    };

    // Generate the broker key
    let broker_unlock_key = EncryptKey::generate(sudo_read.size());
    let broker_key = EncryptKey::xor(sudo_read, &broker_unlock_key);

    // If it already exists then fail
    let wallet_key_entropy = format!("wallet://{}/{}", identity, wallet_name);
    let wallet_key = PrimaryKey::from(wallet_key_entropy);
    if dio.exists(&wallet_key).await {
        eprintln!("Wallet ({}) already exists (with same key).", wallet_name);
        std::process::exit(1);
    }

    // Create the new wallet
    let mut wallet = dio.store_with_key(
        Wallet {
            name: wallet_name.clone(),
            inbox: DaoVec::default(),
            bags: DaoMap::default(),
            history: DaoVec::default(),
            gst_country,
            broker_key,
            broker_unlock_key,
        },
        wallet_key,
    )?;

    // Set its permissions and attach it to the parent
    wallet.auth_mut().read = ReadOption::from_key(sudo_read);
    wallet.auth_mut().write = WriteOption::Inherit;
    wallet.attach_orphaned_ext(&parent_key, WALLET_COLLECTION_ID)?;

    // Now add the history
    let wallet = {
        let mut api = crate::api::build_api_accessor(&dio, wallet, auth.clone(), None, registry).await;
        if let Err(err) = api
            .record_activity(HistoricActivity::WalletCreated(activities::WalletCreated {
                when: chrono::offset::Utc::now(),
                by: api.user_identity(),
            }))
            .await
        {
            error!("Error writing activity: {}", err);
        }
        api.wallet
    };

    Ok(wallet)
}

pub async fn get_wallet<A>(
    purpose: &dyn OptsPurpose<A>,
    dio: &Arc<DioMut>,
    identity: &String,
) -> std::result::Result<DaoMut<Wallet>, AteError>
where
    A: Clone,
{
    // Make sure the parent exists
    let parent_key = PrimaryKey::from(identity.clone());
    debug!("parent_key={}", parent_key);
    if dio.exists(&parent_key).await == false {
        eprintln!("The parent user or group does not exist in the chain-or-trust.");
        std::process::exit(1);
    }

    // Grab a reference to the wallet
    let wallet_name = get_wallet_name(purpose)?;
    let mut wallet_vec = DaoVec::<Wallet>::new_orphaned_mut(dio, parent_key, WALLET_COLLECTION_ID);
    let wallet = wallet_vec
        .iter_mut()
        .await?
        .into_iter()
        .filter(|a| a.name.eq_ignore_ascii_case(wallet_name.as_str()))
        .next();

    let wallet = match wallet {
        Some(a) => a,
        None => {
            eprintln!(
                "Wallet ({}) does not exist - you must first 'create' the wallet before using it.",
                wallet_name
            );
            std::process::exit(1);
        }
    };

    Ok(wallet)
}

pub(crate) struct PurposeContextPrelude<A>
where
    A: Clone,
{
    pub action: A,
    #[allow(dead_code)]
    pub session: AteSessionType,
    pub identity: String,
    pub registry: Arc<Registry>,
    #[allow(dead_code)]
    pub chain_key: ChainKey,
    #[allow(dead_code)]
    pub chain: ChainGuard,
    pub dio: Arc<DioMut>,
}

pub(crate) struct PurposeContext<A>
where
    A: Clone,
{
    pub inner: PurposeContextPrelude<A>,
    pub api: TokApi,
}

impl<A> Deref for PurposeContext<A>
where
    A: Clone,
{
    type Target = PurposeContextPrelude<A>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<A> DerefMut for PurposeContext<A>
where
    A: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<A> PurposeContextPrelude<A>
where
    A: Clone,
{
    pub async fn new(
        purpose: &dyn OptsPurpose<A>,
        token_path: &str,
        auth_url: &url::Url,
        sudo: bool,
    ) -> Result<PurposeContextPrelude<A>, CoreError> {
        // Build a session with all the needed permissions
        let (action, session) =
            session_with_permissions(purpose, token_path, auth_url, sudo).await?;

        // Compute the identity of the requesting user or group
        let identity = get_identity(purpose, &session).await?;

        // Open the chain
        let registry = ate::mesh::Registry::new(&ate_auth::helper::conf_auth())
            .await
            .keep_alive(Duration::from_secs(10))
            .cement();
        let chain_key = chain_key_4hex(&identity, Some("redo"));
        debug!("chain_url={}", auth_url);
        debug!("chain_key={}", chain_key);
        let chain = registry.open(&auth_url, &chain_key).await?;

        // Open the DIO
        let dio = chain.dio_trans(&session, TransactionScope::Full).await;

        Ok(PurposeContextPrelude {
            action,
            session,
            identity,
            registry,
            chain_key,
            chain,
            dio,
        })
    }
}

impl<A> PurposeContext<A>
where
    A: Clone,
{
    pub async fn new(
        purpose: &dyn OptsPurpose<A>,
        token_path: &str,
        auth_url: &url::Url,
        db_url: Option<&url::Url>,
        sudo: bool,
    ) -> Result<PurposeContext<A>, CoreError> {
        let inner = PurposeContextPrelude::new(purpose, token_path, auth_url, sudo).await?;

        // Create the API to the wallet
        let wallet = get_wallet(purpose, &inner.dio, &inner.identity).await?;
        let api = build_api_accessor(&inner.dio, wallet, auth_url.clone(), db_url.map(|a| a.clone()), &inner.registry).await;

        Ok(PurposeContext { inner, api })
    }
}
