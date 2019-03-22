package com.tokera.ate.dao.io;

import com.tokera.server.api.commons.LoggerHook;
import com.tokera.server.api.dao.*;
import com.tokera.server.api.dao.cloud.*;
import com.tokera.server.api.dao.ids.*;
import com.tokera.server.api.delegate.MegaDelegate;
import com.tokera.server.api.dto.EffectivePermissions;
import com.tokera.server.api.dto.msg.*;
import com.tokera.server.api.exception.ClientApiException;
import com.tokera.server.api.exception.ErrorCode;
import com.tokera.server.api.qualifiers.LoggingEngine;
import com.tokera.server.api.repositories.DataContainer;
import com.tokera.server.api.units.*;
import org.checkerframework.checker.nullness.qual.EnsuresNonNullIf;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.util.*;
import java.util.stream.Collectors;

@ApplicationScoped
public class CloudIO
{
    private MegaDelegate d = MegaDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @LoggingEngine
    private ICloudIO back;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public CloudIO() {
    }
    
    public boolean merge(BaseDao t) {
        return back.merge(t);
    }

    public boolean merge(MessagePublicKeyDto t) {
        return back.merge(t);
    }

    public boolean merge(MessageEncryptTextDto t) {
        return back.merge(t);
    }
    
    public void mergeLater(BaseDao t) {
        back.mergeLater(t);
    }
    
    public void mergeDeferred() {
        back.mergeDeferred();
    }
    
    public void clearDeferred() {
        back.clearDeferred();
    }

    public void clearCache(@DaoId UUID id) {
        back.clearCache(id);
    }

    public boolean remove(BaseDao t) {
        return back.remove(t);
    }

    public void removeLater(BaseDao t) {
        back.removeLater(t);
    }

    public <T extends BaseDao> boolean remove(@DaoId UUID id, Class<T> type) {
        return back.remove(id, type);
    }

    public void cache(BaseDao entity) {
        back.cache(entity);
    }
    
    public EffectivePermissions perms(BaseDao obj) {
        return perms(obj.getId(), obj.getParentId(), true);
    }

    public EffectivePermissions perms(@DaoId UUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged) {
        return back.perms(id, parentId, usePostMerged);
    }

    public void warm() {
        back.warm();
    }

    public void sync() { back.sync(); }

    public boolean sync(MessageSyncDto sync) { return back.sync(sync); }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        return back.publicKeyOrNull( hash);
    }

    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(@Nullable @DaoId UUID id) {
        if (id == null) return false;
        return back.exists(id);
    }

    public boolean ethereal() {
        return back.ethereal();
    }

    public boolean everExisted(@Nullable @DaoId UUID id){
        if (id == null) return false;
        return back.everExisted(id);
    }

    public boolean immutable(@DaoId UUID id) {
        return back.immutable(id);
    }
    
    private <T extends BaseDao> T get(@DaoId UUID id, Class<T> type) {
        try {
            BaseDao ret = back.getOrNull(id);
            if (ret == null) {
                throw new ClientApiException(type.getSimpleName() + " not found (id=" + id + ")",
                        ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
            }
            if (ret.getClass() != type) {
                throw new ClientApiException(type.getSimpleName() + " of the wrong type (id=" + id + ", actual=" + ret.getClass().getSimpleName() + ", expected=" + type.getSimpleName() + ")",
                        ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
            }
            return (T)ret;
        } catch (ClassCastException ex) {
            throw new ClientApiException(type.getSimpleName() + " of the wrong type (id=" + id + ")",
                    ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND, ex);
        }
    }

    private BaseDao get(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret == null) {
            throw new ClientApiException("Object data (id=" + id + ") not found",
                    ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
        }
        return ret;
    }

    public BaseDao getExceptional(@DaoId UUID id) {
        return get(id);
    }

    public DataContainer getRaw(@DaoId UUID id)
    {
        DataContainer ret = back.getRawOrNull(id);
        if (ret == null) {
            throw new ClientApiException("Object data (id=" + id + ") not found",
                    ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
        }
        return ret;
    }

    public @Nullable DataContainer getRawOrNull(@DaoId UUID id)
    {
        return back.getRawOrNull(id);
    }

    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        return back.getHistory(id, clazz);
    }

    public BaseDao getVersion(@DaoId UUID id, MessageMetaDto meta) {
        BaseDao ret = back.getVersionOrNull(id, meta);
        if (ret == null) {
            throw new ClientApiException("Object version data (id=" + id + ") not found",
                    ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
        }
        return ret;
    }
    
    public MessageDataDto getVersionMsg(@DaoId UUID id, MessageMetaDto meta) {
        MessageDataDto ret = back.getVersionMsgOrNull(id, meta);
        if (ret == null) {
            throw new ClientApiException("Object version message (id=" + id + ") not found",
                    ErrorCode.INTERNAL_SERVER_ERROR, Response.Status.NOT_FOUND);
        }
        return ret;
    }

    public <T extends BaseDao> Set<T> getAll() {
        return back.getAll();
    }

    public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        return back.getAll(type);
    }

    public <T extends BaseDao> List<DataContainer> getAllRaw() { return back.getAllRaw(); }

    public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type) { return back.getAllRaw(type); }
    
    private <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type) {
        return back.getMany(ids, type);
    }

    public @Nullable BaseDao getParent(@Nullable BaseDao entity)
    {
        if (entity == null) return null;

        @DaoId UUID parentId = entity.getParentId();
        if (parentId == null) return null;
        if (this.exists(parentId) == false) return null;

        if (parentId.equals(entity.getId())) return null;
        return back.getOrNull(parentId);
    }

    public @Nullable IParams getDaoParams(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret instanceof IParams) {
            return (IParams)ret;
        }
        return null;
    }

    public @Nullable IRights getDaoRights(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret instanceof IRights) {
            return (IRights)ret;
        }
        return null;
    }

    public @Nullable IRoles getDaoRoles(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret instanceof IRoles) {
            return (IRoles)ret;
        }
        return null;
    }

    public Account getAccountById(@AccountId UUID accId) {
        return this.get(accId, Account.class);
    }

    public Account getAccountById(@AccountId UUID accId, @TopicName String topic) {
        d.contextShare.pushTopicName(topic);
        try {
            return this.get(accId, Account.class);
        } finally {
            d.contextShare.popTopicName();
        }
    }

    public List<Account> getAccountsByIds(Collection<@AccountId UUID> ids) {
        return this.getMany(ids, Account.class);
    }

    public AccountRole getAccountRoleById(@AccountRoleId UUID id) {
        return this.get(id, AccountRole.class);
    }

    public Address getAddressById(@AddressId UUID id) {
        return this.get(id, Address.class);
    }

    public ClusterListing getClusterListingById(@ClusterListingId UUID id) {
        return this.get(id, ClusterListing.class);
    }

    public ClusterRuntimeContract getClusterRuntimeContractById(@ClusterRuntimeContractId UUID id) {
        return this.get(id, ClusterRuntimeContract.class);
    }

    public CommandExecutionRequest getCommandExecutionRequestById(@CommandExecutionRequestId UUID id) {
        return this.get(id, CommandExecutionRequest.class);
    }

    public Company getCompanyById(@CompanyId UUID id) {
        return this.get(id, Company.class);
    }

    public CreditCard getCreditCardById(@CreditCardId UUID id) {
        return this.get(id, CreditCard.class);
    }

    public CurrencyExchange getCurrencyExchangeById(@CurrencyExchangeId UUID id) {
        return this.get(id, CurrencyExchange.class);
    }

    public DomainCertificate getDomainCertificateById(@DomainCertificateId UUID id) {
        return this.get(id, DomainCertificate.class);
    }

    public DownTimeHistory getDownTimeHistoryById(@DownTimeHistoryId UUID id) {
        return this.get(id, DownTimeHistory.class);
    }

    public ExternalBankAccount getExternalBankAccountById(@ExternalBankAccountId UUID id) {
        return this.get(id, ExternalBankAccount.class);
    }

    public GitRepo getGitRepoById(@GitRepoId UUID id) {
        return this.get(id, GitRepo.class);
    }

    public HoldingWallet getHoldingWalletById(@HoldingWalletId UUID id) {
        return this.get(id, HoldingWallet.class);
    }

    public Invoice getInvoiceById(@InvoiceId UUID id) {
        return this.get(id, Invoice.class);
    }

    public InvoiceItem getInvoiceItemById(@InvoiceItemId UUID id) {
        return this.get(id, InvoiceItem.class);
    }

    public ListingAdvert getListingAdvertById(@ListingAdvertId UUID id) {
        return this.get(id, ListingAdvert.class);
    }

    public NameEntry getNameEntryById(@NameEntryId UUID id) {
        return this.get(id, NameEntry.class);
    }

    public NameEntryBridge getNameEntryBridgeById(@NameEntryBridgeId UUID id) {
        return this.get(id, NameEntryBridge.class);
    }

    public NameEntryIPAddress getNameEntryIPAddressById(@NameEntryIPAddressId UUID id) {
        return this.get(id, NameEntryIPAddress.class);
    }

    public PhysicalDrive getPhysicalDriveById(@PhysicalDriveId UUID id) {
        return this.get(id, PhysicalDrive.class);
    }

    public RateCard getRateCardById(@RateCardId UUID id) {
        return this.get(id, RateCard.class);
    }

    public Script getScriptById(@ScriptId UUID id) {
        return this.get(id, Script.class);
    }

    public SignalPoint getSignalPointById(@SignalPointId UUID id) {
        return this.get(id, SignalPoint.class);
    }

    public SshPublicKey getSshPublicKeyById(@SshPublicKeyId UUID id) {
        return this.get(id, SshPublicKey.class);
    }

    public TextFile getTextFileById(@TextFileId UUID id) {
        return this.get(id, TextFile.class);
    }

    public TokeraCluster getTokeraClusterById(@TokeraClusterId UUID id) {
        return this.get(id, TokeraCluster.class);
    }

    public TokeraNode getTokeraNodeById(@TokeraNodeId UUID id) {
        return this.get(id, TokeraNode.class);
    }

    public TokeraNodeRegistration getTokeraNodeRegistrationById(@TokeraNodeRegistrationId UUID id) {
        return this.get(id, TokeraNodeRegistration.class);
    }

    public TokeraRuntime getTokeraRuntimeById(@TokeraRuntimeId UUID id) {
        return this.get(id, TokeraRuntime.class);
    }

    public TokeraRuntimeGroup getTokeraRuntimeGroupById(@TokeraRuntimeGroupId UUID id) {
        return this.get(id, TokeraRuntimeGroup.class);
    }

    public TokeraUser getTokeraUserById(@TokeraUserId UUID id) {
        return this.get(id, TokeraUser.class);
    }

    public Transaction getTransactionById(@TransactionId UUID id) {
        return this.get(id, Transaction.class);
    }

    public UserCertificate getUserCertificateById(@UserCertificateId UUID id) {
        return this.get(id, UserCertificate.class);
    }

    public VirtualAdaptor getVirtualAdaptorById(@VirtualAdaptorId UUID id) {
        return this.get(id, VirtualAdaptor.class);
    }

    public VirtualDisk getVirtualDiskById(@VirtualDiskId UUID id) {
        return this.get(id, VirtualDisk.class);
    }

    public VirtualMachine getVirtualMachineById(@VirtualMachineId UUID id) {
        return this.get(id, VirtualMachine.class);
    }

    public VirtualMachineTemplate getVirtualMachineTemplateById(@VirtualMachineTemplateId UUID id) {
        return this.get(id, VirtualMachineTemplate.class);
    }

    public VirtualNodeContract getVirtualNodeContractById(@VirtualNodeContractId UUID id) {
        return this.get(id, VirtualNodeContract.class);
    }

    public VirtualNodeUsageHistory getVirtualNodeUsageHistoryById(@VirtualNodeUsageHistoryId UUID id) {
        return this.get(id, VirtualNodeUsageHistory.class);
    }

    public VirtualPort getVirtualPortById(@VirtualPortId UUID id) {
        return this.get(id, VirtualPort.class);
    }

    public VirtualSegment getVirtualSegmentById(@VirtualSegmentId UUID id) {
        return this.get(id, VirtualSegment.class);
    }

    public VirtualNetwork getVirtualNetworkById(@VirtualNetworkId UUID id) {
        return this.get(id, VirtualNetwork.class);
    }

    public VirtualNetworkTemplate getVirtualNetworkTemplateById(@VirtualNetworkTemplateId UUID id) {
        return this.get(id, VirtualNetworkTemplate.class);
    }

    public VirtualStorage getVirtualStorageById(@VirtualStorageId UUID id) {
        return this.get(id, VirtualStorage.class);
    }

    public VirtualStorageTemplate getVirtualStorageTemplateById(@VirtualStorageTemplateId UUID id) {
        return this.get(id, VirtualStorageTemplate.class);
    }

    public Wallet getWalletById(@WalletId UUID id) {
        return this.get(id, Wallet.class);
    }

    public Wallet getWalletByIdOrThrow(@Nullable @WalletId UUID id) {
        if (id == null) throw new WebApplicationException("Wallet is not valid");
        return this.get(id, Wallet.class);
    }

    public Zone getZoneById(@ZoneId UUID id) {
        return this.get(id, Zone.class);
    }

    public @Nullable Wallet getWalletForAccount(Account acc) {
        if (acc.walletId == null) return null;
        return this.get(acc.walletId, Wallet.class);
    }

    public List<DomainCertificate> getDomainCertificatesForZone(Zone zone) {
        return this.getMany(zone.domainCertificateIds, DomainCertificate.class);
    }

    public List<ClusterRuntimeContract> getUnprocessedClusterRuntimeContracts(int daysSinceEpoch, int max) {

        List<ClusterRuntimeContract> ret = this.getAll(ClusterRuntimeContract.class)
                .stream()
                .filter(a -> a.daysSinceEpoch != null && a.daysSinceEpoch != daysSinceEpoch)
                .limit(max)
                .collect(Collectors.toList());
        
        return ret;
    }

    public @Nullable TokeraNodeRegistration getTokeraNodeRegistrationByHardwareHash(@Hash String hash) {
        return this.getAll(TokeraNodeRegistration.class)
                .stream()
                .filter(a -> a.hardwareHash.equals(hash))
                .findFirst()
                .orElse(null);
    }

    public @Nullable TokeraNodeRegistration getTokeraNodeRegistrationByRegistrationCode(@Alias String code) {
        return this.getAll(TokeraNodeRegistration.class)
                .stream()
                .filter(a -> a.registrationCode.equals(code))
                .findFirst()
                .orElse(null);
        
    }
    
    public @Nullable VirtualNetwork getVirtualNetworkByVirtualNetworkTemplate(VirtualNetworkTemplate vnt) {
        if (vnt.virtualNetwork == null) return null;
        return this.get(vnt.virtualNetwork, VirtualNetwork.class);
    }

    public List<VirtualMachine> getVirtualMachinesByVirtualNodeContractId(@VirtualNodeContractId UUID virtualNodeContractId) {
        return this.getVirtualMachinesByVirtualNodeContract(this.get(virtualNodeContractId, VirtualNodeContract.class));
    }

    public List<VirtualMachine> getVirtualMachinesByVirtualNodeContract(VirtualNodeContract vnc) {
        return this.getMany(vnc.virtualMachines, VirtualMachine.class);
    }

    public List<VirtualMachine> getVirtualMachinesByOwnerAccountId(@AccountId UUID ownerAccountId) {
        Account account = this.get(ownerAccountId, Account.class);
        return this.getVirtualMachinesByOwnerAccount(account);
    }

    public List<VirtualMachine> getVirtualMachinesByOwnerAccount(Account account) {
        Set<@VirtualMachineId UUID> ids = new HashSet<>();
        for (VirtualMachineTemplate vmt : this.getVirtualMachineTemplatesByOwnerAccount(account)) {
            ids.addAll(vmt.virtualMachines);
        }
        return this.getMany(ids, VirtualMachine.class);
    }
    
    public List<VirtualMachine> getVirtualMachinesByTokeraNode(TokeraNode tokeraNode) {
        return this.getMany(tokeraNode.virtualMachines, VirtualMachine.class);
    }

    public List<VirtualMachine> getRelatedVirtualMachinesByTokeraNode(TokeraNode tokeraNode) {
        List<VirtualMachine> vms = new ArrayList<>();

        Account nodAcc = this.get(tokeraNode.ownerAccountId, Account.class);
        for (VirtualMachine vm : this.getVirtualMachinesByOwnerAccount(nodAcc)) {
            if (vms.contains(vm) == false)
                vms.add(vm);
        }

        TokeraNode nod = tokeraNode;
        for (VirtualMachine vm : this.getVirtualMachinesByTokeraNode(nod)) {
            if (vms.contains(vm) == false)
                vms.add(vm);
        }

        TokeraCluster nodCluster = this.get(tokeraNode.clusterId, TokeraCluster.class);
        for (VirtualMachine vm : this.getVirtualMachinesByCluster(nodCluster)) {
            if (vms.contains(vm) == false)
                vms.add(vm);
        }

        return vms;
    }
    
    public List<VirtualMachine> getVirtualMachinesByClusterId(@TokeraClusterId UUID clusterId) {
        TokeraCluster cluster = this.get(clusterId, TokeraCluster.class);
        return this.getVirtualMachinesByCluster(cluster);
    }
    
    public List<VirtualMachine> getVirtualMachinesByCluster(TokeraCluster cluster) {
        return this.getMany(cluster.virtualMachines, VirtualMachine.class);
    }
    
    public List<VirtualMachine> getVirtualMachinesByVirtualMachineTemplate(VirtualMachineTemplate vmt) {
        return this.getMany(vmt.virtualMachines, VirtualMachine.class);
    }
    
    public List<VirtualMachine> getVirtualMachinesByClusterRuntimeContract(ClusterRuntimeContract crc) {
        List<VirtualMachine> ret = new ArrayList<>();
        for (VirtualMachine vm : this.getVirtualMachinesByRuntimeId(crc.runtimeId)) {
            if (Objects.equal(crc.getId(), vm.clusterRuntimeContract))
                ret.add(vm);
        }
        return ret;
    }
    
    public List<VirtualMachine> getVirtualMachinesByRuntimeId(@TokeraRuntimeId UUID runtimeId) {
        return this.getVirtualMachinesByRuntime(this.get(runtimeId, TokeraRuntime.class));
    }
    
    public List<VirtualMachine> getVirtualMachinesByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualMachine> ret = getVirtualMachinesByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }

    public List<VirtualMachineTemplate> getVirtualMachineTemplatesByOwnerAccount(Account account) {
        Set<@VirtualMachineTemplateId UUID> ids = new HashSet<>();
        for (TokeraRuntime runtime : this.getTokeraRuntimesByAccount(account)) {
            ids.addAll(runtime.virtualMachineTemplates);
        }
        return this.getMany(ids, VirtualMachineTemplate.class);
    }
    
    public List<VirtualMachineTemplate> getVirtualMachineTemplatesByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.virtualMachineTemplates, VirtualMachineTemplate.class);
    }

    public List<VirtualStorage> getVirtualStoragesByOwnerAccount(Account account) {
        Set<@VirtualStorageId UUID> ids = new HashSet<>();
        for (VirtualStorageTemplate vst : this.getVirtualStorageTemplatesByOwnerAccount(account)) {
            ids.addAll(vst.virtualStorages);
        }
        return this.getMany(ids, VirtualStorage.class);
    }
    
    public List<VirtualStorage> getVirtualStoragesByClusterId(@TokeraClusterId UUID clusterId) {
        TokeraCluster cluster = this.get(clusterId, TokeraCluster.class);
        return this.getVirtualStoragesByCluster(cluster);
    }
    
    public List<VirtualStorage> getVirtualStoragesByCluster(TokeraCluster cluster) {
        return this.getMany(cluster.virtualStorages, VirtualStorage.class);
    }
    
    public List<VirtualStorage> getVirtualStoragesByNode(TokeraNode node) {
        return this.getVirtualStoragesByClusterId(node.clusterId)
                .stream()
                .filter(vs -> node.getId().equals(vs.tokeraNode))
                .collect(Collectors.toList());
    }
    
    public List<VirtualStorage> getVirtualStoragesByVirtualStorageTemplate(VirtualStorageTemplate vst) {
        return this.getMany(vst.virtualStorages, VirtualStorage.class);
    }
    
    public List<VirtualMachineTemplate> getVirtualMachineTemplatesByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualMachineTemplate> ret = getVirtualMachineTemplatesByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<VirtualStorage> getVirtualStoragesByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualStorage> ret = getVirtualStoragesByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<VirtualStorageTemplate> getVirtualStorageTemplatesByOwnerAccount(Account account) {
        Set<@VirtualStorageTemplateId UUID> ids = new HashSet<>();
        for (TokeraRuntime runtime : this.getTokeraRuntimesByAccount(account)) {
            ids.addAll(runtime.virtualStorageTemplates);
        }
        return this.getMany(ids, VirtualStorageTemplate.class);
    }
    
    public List<VirtualStorageTemplate> getVirtualStorageTemplatesByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.virtualStorageTemplates, VirtualStorageTemplate.class);
    }
    
    public List<VirtualStorageTemplate> getVirtualStorageTemplatesByCluster(TokeraCluster cluster) {
        return this.getMany(cluster.virtualStorageTemplates, VirtualStorageTemplate.class);
    }
    
    public List<VirtualStorageTemplate> getVirtualStorageTemplatesByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualStorageTemplate> ret = getVirtualStorageTemplatesByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<VirtualNetworkTemplate> getVirtualNetworkTemplatesByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.virtualNetworkTemplates, VirtualNetworkTemplate.class);
    }
    
    public List<VirtualNetworkTemplate> getVirtualNetworkTemplatesByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualNetworkTemplate> ret = getVirtualNetworkTemplatesByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<ClusterRuntimeContract> getClusterRuntimeContractsByOwnerAccount(Account account) {
        Set<@ClusterRuntimeContractId UUID> ids = new HashSet<>();
        for (TokeraRuntime runtime : this.getTokeraRuntimesByAccount(account)) {
            ids.addAll(runtime.clusterRuntimeContracts);
        }
        return this.getMany(ids, ClusterRuntimeContract.class);
    }
    
    public List<ClusterRuntimeContract> getClusterRuntimeContractsBySupplierAccount(Account acc) {
        return this.getMany(acc.supplyingClusterRuntimeContracts, ClusterRuntimeContract.class);
    }
    
    public Set<ClusterListing> getClusterListings() {
        return this.getAll(ClusterListing.class);
    }
    
    public List<ClusterListing> getClusterListingsByCluster(TokeraCluster cluster) {
        List<ClusterListing> ret = new ArrayList<>();
        for (ClusterListing listing : getClusterListings()) {
            if (listing.clusterId.equals(cluster.getId())) {
                ret.add(listing);
            }
        }
        return ret;
    }
    
    public List<ClusterListing> getClusterListingsByAccount(Account acc) {
        List<ClusterListing> ret = new ArrayList<>();
        for (ClusterListing listing : getClusterListings()) {
            if (listing.ownerAccountId.equals(acc.getId())) {
                ret.add(listing);
            }
        }
        return ret;
    }
    
    public List<ClusterRuntimeContract> getClusterRuntimeContractsByRuntime(TokeraRuntime run) {
        return this.getMany(run.clusterRuntimeContracts, ClusterRuntimeContract.class);
    }
    
    public List<ClusterRuntimeContract> getClusterRuntimeContractsByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<ClusterRuntimeContract> ret = getClusterRuntimeContractsByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<TokeraRuntimeGroup> getTokeraRuntimeGroupsByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.runtimeGroups.values(), TokeraRuntimeGroup.class);
    }
    
    public List<TokeraRuntimeGroup> getTokeraRuntimeGroupsByParent(TokeraRuntimeGroup parent) {
        TokeraRuntime runtime = this.get(parent.runtimeId, TokeraRuntime.class);
        List<TokeraRuntimeGroup> ret = getTokeraRuntimeGroupsByRuntime(runtime);
        return ret.stream()
                .filter(a -> parent.getId().equals(a.parentGroupId))
                .collect(Collectors.toList());
    }
    
    public @Nullable TokeraRuntimeGroup getTokeraRuntimeGroupByPath(TokeraRuntime runtime, @Alias String fullpath) {
        if (fullpath.startsWith("/") == false) {
            fullpath = "/" + fullpath;
        }

        if (runtime.runtimeGroups.containsKey(fullpath) == false) {
            return null;
        }
        
        @TokeraRuntimeGroupId UUID groupId = runtime.runtimeGroups.get(fullpath);
        if (groupId == null) return null;
        return this.get(groupId, TokeraRuntimeGroup.class);
    }

    public TokeraRuntimeGroup getRootRuntimeGroupForRuntime(TokeraRuntime runtime) {
        if (runtime.runtimeGroups.containsKey("/") == false) {
            throw new WebApplicationException("Runtime does not have a root group.");
        }
        @TokeraRuntimeGroupId UUID rootId = runtime.runtimeGroups.get("/");
        if (rootId == null) {
            throw new WebApplicationException("Runtime does not have a root group.");
        }
        return this.getTokeraRuntimeGroupById(rootId);
    }
    
    public List<VirtualDisk> getVirtualDisksByTokeraNode(TokeraNode node) {
        
        ArrayList<@VirtualDiskId UUID> ids = new ArrayList<>();
        
        for (VirtualMachine vm : this.getVirtualMachinesByTokeraNode(node))
        {
            if (vm.diskBios!= null && ids.contains(vm.diskBios) == false) {
                ids.add(vm.diskBios);
            }
            if (vm.diskCdrom != null && ids.contains(vm.diskCdrom) == false) {
                ids.add(vm.diskCdrom);
            }
            if (vm.diskInitrd != null && ids.contains(vm.diskInitrd) == false) {
                ids.add(vm.diskInitrd);
            }
            if (vm.diskKernel != null && ids.contains(vm.diskKernel) == false) {
                ids.add(vm.diskKernel);
            }
            for (UUID id : vm.virtualDisks) {
                if (ids.contains(id) == false)
                    ids.add(id);
            }
        }
        return getMany(ids, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByVirtualStorage(VirtualStorage vs) {
        return getMany(vs.virtualDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getBackingDisksByVirtualStorage(VirtualStorage vs) {
        return getMany(vs.backingDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByCluster(TokeraCluster cluster) {
        return getMany(cluster.virtualDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByClusterId(@TokeraClusterId UUID id) {
        return getVirtualDisksByCluster(this.get(id, TokeraCluster.class));
    }
    
    public List<VirtualDisk> getVirtualDisksByAccount(Account acc) {
        Set<@VirtualDiskId UUID> ids = new HashSet<>();
        for (TokeraRuntime runtime : this.getTokeraRuntimesByAccount(acc)) {
            ids.addAll(runtime.virtualDisks);
        }
        return this.getMany(ids, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByVirtualMachine(VirtualMachine vm) {
        return this.getMany(vm.virtualDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByVirtualNodeContract(VirtualNodeContract vnc) {
        return this.getMany(vnc.virtualDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.virtualDisks, VirtualDisk.class);
    }
    
    public List<VirtualDisk> getVirtualDisksByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualDisk> ret = getVirtualDisksByRuntime(runtime);
        return ret.stream()
                .filter(a -> group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<VirtualNetwork> getVirtualNetworksByAccount(Account acc) {
        Set<@VirtualNetworkId UUID> ids = new HashSet<>();

        for (VirtualNetworkTemplate vnt : this.getVirtualNetworkTemplatesByAccount(acc)) {
            @VirtualMachineId UUID virtualNetworkId = vnt.virtualNetwork;
            if (virtualNetworkId != null) {
                ids.add(virtualNetworkId);
            }
        }
        for (TokeraCluster cluster : this.getClustersByAccount(acc)) {
            ids.add(cluster.virtualNetworkId);
        }
        return this.getMany(ids, VirtualNetwork.class);
    }

    public List<VirtualNetwork> getVirtualNetworksByTokeraNode(TokeraNode node) {

        List<VirtualSegment> segments = this.getVirtualSegmentsByTokeraNode(node);

        HashSet<@VirtualNetworkId UUID> ids = new HashSet<>();
        for (VirtualSegment segment : segments) {

            if (ids.contains(segment.virtualNetwork) == false) {
                ids.add(segment.virtualNetwork);
            }

        }

        return this.getMany(ids, VirtualNetwork.class);
    }

    public List<VirtualNetwork> getVirtualNetworksBySegments(Collection<VirtualSegment> segs)
    {
        List<UUID> networkIds = segs.stream().map(s -> s.virtualNetwork).distinct().collect(Collectors.toList());
        return this.getMany(networkIds, VirtualNetwork.class);
    }

    public List<VirtualSegment> getCoreVirtualSegmentsByNetworks(Collection<VirtualNetwork> networks)
    {
        List<UUID> coreIds = new ArrayList<>();
        for (VirtualNetwork net : networks) {
            @VirtualSegmentId UUID coreId = net.coreSegment;
            if (coreId == null) continue;
            coreIds.add(coreId);
        }
        return this.getMany(coreIds, VirtualSegment.class);
    }

    public List<VirtualNetwork> getVirtualNetworksByAccountId(@AccountId UUID accId) {
        Account account = this.get(accId, Account.class);
        return this.getVirtualNetworksByAccount(account);
    }
    
    public List<Zone> getZonesByAccount(Account acc) {
        return this.getMany(acc.zoneDomainIds, Zone.class);
    }
    
    public @Nullable Zone getZoneByAccountOrNull(Account acc, @DomainName String domain) {
        for (Zone zone : this.getMany(acc.zoneDomainIds, Zone.class)) {
            if (domain.equals(zone.domain) == true) {
                return zone;
            }
        }
        return null;
    }

    public @Nullable Zone getZoneByAccountRecursiveOrNull(Account acc, @DomainName String domain) {
        while (domain.length() > 0)
        {
            Zone zone = getZoneByAccountOrNull(acc, domain);
            if (zone != null) return zone;

            int firstDot = domain.indexOf(".");
            if (firstDot == -1) return null;

            if (firstDot + 1 >= domain.length()) break;
            domain = domain.substring(firstDot + 1);
        }
        return null;
    }

    public List<VirtualNetworkTemplate> getVirtualNetworkTemplatesByAccount(Account acc) {
        Set<@VirtualNetworkTemplateId UUID> ids = new HashSet<>();
        for (TokeraRuntime runtime : this.getTokeraRuntimesByAccount(acc)) {
            ids.addAll(runtime.virtualNetworkTemplates);
        }

        return this.getMany(ids, VirtualNetworkTemplate.class);
    }
    
    public List<VirtualNetwork> getVirtualNetworksByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        List<VirtualNetwork> ret = this.getVirtualNetworksByRuntime(runtime);
        return ret.stream()
                .filter(a -> a.runtimeGroup != null && group.getId().equals(a.runtimeGroup))
                .collect(Collectors.toList());
    }
    
    public List<TokeraCluster> getClustersByAccount(Account acc) {
        return this.getMany(acc.clusters, TokeraCluster.class);
    }

    public List<TokeraNode> getTokeraNodesByAccount(Account acc)
    {
        List<@TokeraNodeId UUID> ids = new ArrayList<>();
        for (TokeraCluster cluster : this.getMany(acc.clusters, TokeraCluster.class)) {
            ids.addAll(cluster.tokeraNodes);
        }
        return this.getMany(ids, TokeraNode.class);
    }

    public List<CreditCard> getCreditCardsForWallet(Wallet wallet) {
        return this.getMany(wallet.creditCards, CreditCard.class);
    }

    public List<Transaction> getTransactionsForWallet(Wallet wallet) {
        return this.getMany(wallet.transactions, Transaction.class);
    }

    public List<Transaction> getTransactionsForInvoice(Invoice invoice) {
        return this.getMany(invoice.transactionCollection, Transaction.class);
    }

    public List<Invoice> getInvoicesForWallet(Wallet wallet) {
        return this.getMany(wallet.invoices, Invoice.class);
    }
    
    public List<TokeraRuntime> getTokeraRuntimesByAccount(Account acc) {
        return this.getMany(acc.runtimeIds, TokeraRuntime.class);
    }
    
    public @Nullable TokeraRuntime getTokeraRuntimeByAccount(Account acc, @DomainName String zoneName) {
        for (TokeraRuntime run : this.getTokeraRuntimesByAccount(acc)) {
            if (zoneName.equals(run.name) == true) {
                return run;
            }
        }
        return null;
    }

    public List<TokeraNode> getTokeraNodesByCluster(TokeraCluster cluster) {
        return this.getMany(cluster.tokeraNodes, TokeraNode.class);
    }
    
    public List<TokeraNode> getTokeraNodesByClusterId(@TokeraClusterId UUID id) {
        return this.getTokeraNodesByCluster(this.get(id, TokeraCluster.class));
    }

    public List<VirtualSegment> getVirtualSegmentsByNetwork(VirtualNetwork network) {
        return this.getMany(network.virtualSegments, VirtualSegment.class);
    }

    public List<VirtualSegment> getVirtualSegmentsByAccount(Account acc) {
        List<@VirtualSegmentId UUID> ids = new ArrayList<>();
        for (VirtualNetwork net : getVirtualNetworksByAccount(acc)) {
            ids.addAll(net.virtualSegments);
        }        
        return this.getMany(ids, VirtualSegment.class);
    }

    public List<VirtualSegment> getVirtualSegmentsByTokeraNode(TokeraNode node) {

        // Build the return list
        HashSet<@VirtualSegmentId UUID> ids = new HashSet<>();

        // Get all the virtual networks
        List<VirtualMachine> vms = this.getVirtualMachinesByTokeraNode(node);
        for (VirtualMachine vm : vms)
        {
            // Grab all the virtual adaptors for this VM
            List<VirtualPort> ports = this.getVirtualPortsByVirtualMachine(vm);
            for (VirtualPort port : ports) {
                if (ids.contains(port.virtualSegment) == false) {
                    ids.add(port.virtualSegment);
                }
            }
        }

        // Load the virtual cluster for this Tokera Node
        TokeraCluster cluster = this.getTokeraClusterById(node.clusterId);
        if (ids.contains(cluster.virtualSegmentId) == false) {
            ids.add(cluster.virtualSegmentId);
        }

        // When a single segment from a virtual network is added then all its siblings must also be added
        List<VirtualSegment> segments = this.getMany(ids, VirtualSegment.class);
        for (VirtualSegment segment : segments)
        {
            // Load the virtual network
            VirtualNetwork network = this.get(segment.virtualNetwork, VirtualNetwork.class);
            if (network.coreSegment != null) {
                if (ids.contains(network.coreSegment) == false) {
                    ids.add(network.coreSegment);
                }
            }
        }

        return this.getMany(ids, VirtualSegment.class);
    }

    public List<VirtualSegment> getVirtualSegmentsByAccountId(@VirtualSegmentId UUID acc) {
        Account account = this.get(acc, Account.class);
        return this.getVirtualSegmentsByAccount(account);
    }
    
    public List<VirtualSegment> getVirtualSegmentsByRuntimeGroup(TokeraRuntimeGroup group) {
        TokeraRuntime runtime = this.get(group.runtimeId, TokeraRuntime.class);
        return this.getMany(this.getVirtualSegmentIdsByRuntime(runtime), VirtualSegment.class)
                .stream()
                .filter(s -> s.runtimeGroup != null && group.getId().equals(s.runtimeGroup))
                .collect(Collectors.toList());
    }

    public List<VirtualAdaptor> getVirtualAdaptorsByVirtualMachine(VirtualMachine vm) {
        return this.getMany(vm.virtualAdaptorIds, VirtualAdaptor.class);
    }
    
    public List<VirtualPort> getVirtualPortsByVirtualMachine(VirtualMachine vm) {
        List<@VirtualPortId UUID> ports = new ArrayList<>();
        for (VirtualAdaptor adp : this.getMany(vm.virtualAdaptorIds, VirtualAdaptor.class)) {
            ports.add(adp.virtualPort);
        }
        return this.getMany(ports, VirtualPort.class)
                .stream()
                .sorted((a, b) -> a.macAddress.compareTo(b.macAddress))
                .collect(Collectors.toList());
    }
    
    public List<VirtualPort> getVirtualPortsByVirtualSegment(VirtualSegment segment) {
        return this.getMany(segment.virtualPorts, VirtualPort.class);
    }
    
    public List<VirtualNodeContract> getVirtualNodeContractsByOwnerAccount(Account acc) {
        return this.getAll(VirtualNodeContract.class)
                .stream()
                .filter(vnc -> vnc.ownerAccountId.equals(acc.getId()))
                .collect(Collectors.toList());
    }
    
    public List<VirtualNodeContract> getVirtualNodeContractsByClusterRuntimeContract(ClusterRuntimeContract crc) {
        List<VirtualNodeContract> contracts = this.getVirtualNodeContractsByRuntimeId(crc.runtimeId);
        List<VirtualNodeContract> ret = new ArrayList<>();
        for (VirtualNodeContract contract : contracts) {
            if (Objects.equal(contract.clusterRuntimeContract, crc.getId()) == true) {
                ret.add(contract);
            }
        }
        return ret;
    }
    
    public List<VirtualNodeContract> getVirtualNodeContractsByRuntimeId(@TokeraRuntimeId UUID id) {
        return this.getVirtualNodeContractsByRuntime(this.get(id, TokeraRuntime.class));
    }
    
    public List<VirtualNodeContract> getVirtualNodeContractsByRuntime(TokeraRuntime runtime) {
        return this.getMany(runtime.virtualNodeContracts, VirtualNodeContract.class);
    }

    public List<PhysicalDrive> getPhysicalDrivesByAccount(Account acc) {
        List<PhysicalDrive> ret = new ArrayList<>();
        for (TokeraCluster cluster : this.getClustersByAccount(acc)) {
            ret.addAll(this.getPhysicalDrivesByCluster(cluster));
        }
        return ret;
    }

    public List<PhysicalDrive> getPhysicalDrivesByCluster(TokeraCluster cluster) {
        return this.getMany(cluster.physicalDrives, PhysicalDrive.class);
    }
    
    public List<PhysicalDrive> getPhysicalDrivesByTokeraNode(TokeraNode node) {
        return this.getMany(node.physicalDrives, PhysicalDrive.class);
    }

    public String generateMacAddress(IMacGenerator macGenerator) {
        String mac = generateMacAddressOrNull(macGenerator);
        if (mac == null) throw new WebApplicationException("Failed to generate a MAC address.", Response.Status.INTERNAL_SERVER_ERROR);
        return mac;
    }

    public @Nullable String generateMacAddressOrNull(IMacGenerator macGenerator) {
        return generateMacAddressOrNull(macGenerator.getMacRanges());
    }

    public @Nullable String generateMacAddressOrNull(VirtualMacRange range)
    {
        // If all the MAC ranges are full then fail
        if (range.isFull == true) {
            return null;
        }

        // Determine the MAC range
        long macStart = Long.parseLong(range.macStart, 16) * 16777216L;
        long macEnd = Long.parseLong(range.macEnd, 16) * 16777216L;

        // Grab the next address
        long currentSeed = range.seed;
        long macSeed = macStart + currentSeed;
        String macAddress = String.format("%06X", (int) (macSeed / 16777216L)) + String.format("%06X", (int) (macSeed % 16777216L));

        // Update the range
        range.seed = currentSeed + 1;
        if (macSeed + 1 >= macEnd) {
            range.isFull = true;
        }

        // If it has not yet been saved then we are already done
        if (range.hasSaved == false) {
            return macAddress;
        }

        // Attempt to save the record (if it returns null then another thread got the MAC before us)
        if (this.merge(range) == false) {
            this.clearCache(range.id);
            return null;
        }

        // Success
        return macAddress;
    }

    public @Nullable String generateMacAddressOrNull(List<@VirtualMacRangeId UUID> macRanges) {
        
        // Enter an atomic loop
        int failed = 0;
        String macAddress;
        for (;;)
        {
            List<VirtualMacRange> ret = this.getMany(macRanges, VirtualMacRange.class)
                .stream()
                .collect(Collectors.toList());
            if (ret.size() <= 0) {
                return null;
            }

            VirtualMacRange best = ret.get(0);
            for (VirtualMacRange range : ret) {
                if (range.isFull == true) {
                    continue;
                }
                if (range.seed < best.seed || best.isFull == true) {
                    best = range;
                }
            }

            macAddress = generateMacAddressOrNull(best);
            if (macAddress != null) break;

            failed++;
            if (failed > 10) {
                return null;
            }
        }

        // Return the result
        return macAddress;
    }

    public List<RateCard> getRateCardsByOwnerAccount(Account acc) {
        return this.getAll(RateCard.class)
                .stream()
                .filter(r -> r.ownerAccountId.equals(acc.getId()))
                .collect(Collectors.toList());
    }

    public List<CommandExecutionRequest> getTokeraNodeScriptByTokeraNode(TokeraNode node) {
        return this.getAll(CommandExecutionRequest.class)
                .stream()
                .filter(r -> r.tokeraNodeId.equals(node.getId()))
                .collect(Collectors.toList());
    }

    public List<Invoice> getInvoicesByClusterRuntimeContract(ClusterRuntimeContract contract) {
        return this.getMany(contract.invoices, Invoice.class);
    }
    
    public List<Invoice> getUnpaidInvoices(int max) {
        return this.getAll(Invoice.class)
                .stream()
                .filter(r -> r.isPaid == false)
                .collect(Collectors.toList());
    }

    public @Nullable TokeraUser getUserByThumbprint(@Hash String thumbprint) {
        UserCertificate userCert = this.getUserCertificateByThumbprint(thumbprint);
        if (userCert == null) return null;
        return this.get(userCert.userId, TokeraUser.class);
    }

    public List<SshPublicKey> getUserPublicKeysByUser(TokeraUser user) {
        return this.getMany(user.sshKeyIds, SshPublicKey.class);
    }

    public TokeraUser getUserByEmail(@EmailAddress String email) {
        return getTokeraUserByEmail(email);
    }

    public List<AccountRole> getAccountRolesByAccount(Account acc) {
        return this.getMany(acc.accountRoles, AccountRole.class);
    }
    
    public @Nullable UserCertificate getUserCertificateByThumbprint(@Hash String thumbprint) {
        for (UserCertificate userHash : this.getAll(UserCertificate.class)) {
            if (thumbprint.equals(userHash.thumbprint) == true) {
                return userHash;
            }
        }
        return null;
    }
    
    public TokeraUser getTokeraUserByEmail(@EmailAddress String email) {
        @TokeraUserId UUID id = d.id.generateUserId(email);
        return this.get(id, TokeraUser.class);
    }
    
    public List<AccountRole> getAllRolesForUser(TokeraUser user) {
        return this.getMany(user.accountRoleIds, AccountRole.class);
    }
    
    public List<VirtualNodeContract> getVirtualNodeContractsByTokeraNode(TokeraNode node) {
        List<@VirtualNodeContractId UUID> ids = new ArrayList<>();
        for (VirtualMachine vm : this.getMany(node.virtualMachines, VirtualMachine.class)) {
            ids.add(vm.virtualNodeContractId);
        }
        return this.getMany(ids, VirtualNodeContract.class);
    }

    public List<@VirtualNetworkId UUID> getVirtualNetworkIdsByRuntime(TokeraRuntime runtime) {
        ArrayList<@VirtualNetworkId UUID> ret = new ArrayList<>();
        for (VirtualNetworkTemplate vnt : this.getVirtualNetworkTemplatesByRuntime(runtime)) {
            @VirtualMachineId UUID virtualNetworkId = vnt.virtualNetwork;
            if (virtualNetworkId == null) continue;
            ret.add(virtualNetworkId);
        }
        return ret;
    }

    public List<VirtualNetwork> getVirtualNetworksByRuntime(TokeraRuntime runtime) {
        return this.getMany(getVirtualNetworkIdsByRuntime(runtime), VirtualNetwork.class);
    }

    public List<@VirtualSegmentId UUID> getVirtualSegmentIdsByRuntime(TokeraRuntime runtime) {
        ArrayList<@VirtualSegmentId UUID> ret = new ArrayList<>();
        for (VirtualNetworkTemplate vnt : this.getVirtualNetworkTemplatesByRuntime(runtime)) {
            ret.addAll(vnt.virtualSegments);
        }
        return ret;
    }

    public List<VirtualSegment> getVirtualSegmentsByRuntime(TokeraRuntime runtime) {
        return this.getMany(getVirtualSegmentIdsByRuntime(runtime), VirtualSegment.class);
    }

    public List<@VirtualStorageId UUID> getVirtualStorageIdsByRuntime(TokeraRuntime runtime) {
        ArrayList<@VirtualStorageId UUID> ret = new ArrayList<>();
        for (VirtualStorageTemplate vnt : this.getVirtualStorageTemplatesByRuntime(runtime)) {
            ret.addAll(vnt.virtualStorages);
        }
        return ret;
    }

    public List<VirtualStorage> getVirtualStoragesByRuntime(TokeraRuntime runtime) {
        return this.getMany(getVirtualStorageIdsByRuntime(runtime), VirtualStorage.class);
    }

    public List<@VirtualMachineId UUID> getVirtualMachineIdsByRuntime(TokeraRuntime runtime) {
        ArrayList<@VirtualMachineId UUID> ret = new ArrayList<>();
        for (VirtualMachineTemplate vmt : this.getVirtualMachineTemplatesByRuntime(runtime)) {
            ret.addAll(vmt.virtualMachines);
        }
        return ret;
    }

    public List<VirtualMachine> getVirtualMachinesByRuntime(TokeraRuntime runtime) {
        return this.getMany(getVirtualMachineIdsByRuntime(runtime), VirtualMachine.class);
    }
}
