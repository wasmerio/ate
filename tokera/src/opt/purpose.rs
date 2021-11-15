pub enum Purpose<A>
where
    A: Clone,
{
    Personal {
        wallet_name: String,
        action: A,
    },
    Domain {
        domain_name: String,
        wallet_name: String,
        action: A,
    },
}

pub trait OptsPurpose<A>
where
    A: Clone,
{
    fn purpose(&self) -> Purpose<A>;
}

impl<'a, A> dyn OptsPurpose<A> + 'a
where
    A: Clone,
{
    pub fn fullname(&'a self, my_identity: &'_ str) -> String {
        match self.purpose() {
            Purpose::Personal {
                wallet_name,
                action: _,
            } => format!("{}({})", my_identity, wallet_name),
            Purpose::Domain {
                domain_name,
                wallet_name,
                action: _,
            } => format!("{}({})", domain_name, wallet_name),
        }
    }

    pub fn group_name(&'a self) -> Option<String> {
        match self.purpose() {
            Purpose::Personal {
                wallet_name: _,
                action: _,
            } => None,
            Purpose::Domain {
                domain_name,
                wallet_name: _,
                action: _,
            } => Some(domain_name),
        }
    }

    pub fn wallet_name(&'a self) -> String {
        match self.purpose() {
            Purpose::Personal {
                wallet_name,
                action: _,
            } => wallet_name,
            Purpose::Domain {
                domain_name: _,
                wallet_name,
                action: _,
            } => wallet_name,
        }
    }

    pub fn action(&'a self) -> A {
        match self.purpose() {
            Purpose::Personal {
                wallet_name: _,
                action,
            } => action,
            Purpose::Domain {
                domain_name: _,
                wallet_name: _,
                action,
            } => action,
        }
    }
}
