package com.tokera.ate.io.repo;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataDigestDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.DaoId;
import org.bouncycastle.util.Arrays;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.*;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.BiFunction;

/**
 * This builder class constructs a validator that will check the chain-of-trust rules to make sure the data
 * message is allowed to be accepted or not
 */
final class TrustValidatorBuilder {
    private final AteDelegate d = AteDelegate.get();

    private LoggerHook LOG;
    private @Nullable Map<UUID, @Nullable MessageDataDto> requestTrust;
    private Consumer<Failure> onFailure = null;
    private BiFunction<UUID, Boolean, DataContainer> onGetData = null;
    private Function<UUID, MessageDataHeaderDto> onGetRootOfTrust = null;
    private Function<String, MessagePublicKeyDto> onGetPublicKey = null;

    public TrustValidatorBuilder() {
    }

    /**
     * Adds a logging engine to this validator of a particular type
     */
    public TrustValidatorBuilder withLogger(@Nullable LoggerHook LOG) {
        this.LOG = LOG;
        return this;
    }

    /**
     * Adds a request trust cache to the validator so that it can build a trust tree in memory as it works
     */
    public TrustValidatorBuilder withRequestTrust(Map<UUID, @Nullable MessageDataDto> requestTrust) {
        this.requestTrust = requestTrust;
        return this;
    }

    /**
     * Callback that will be invoked when a validation failure occurs
     */
    public TrustValidatorBuilder withFailureCallback(Consumer<Failure> callback) {
        this.onFailure = callback;
        return this;
    }

    /**
     * Callback thats invoked when the validator needs to lookup another record
     */
    public TrustValidatorBuilder withGetDataCallback(BiFunction<UUID, Boolean, DataContainer> callback) {
        this.onGetData = callback;
        return this;
    }

    /**
     * Callback thats invoked when the validator needs to lookup another record
     */
    public TrustValidatorBuilder withGetRootOfTrust(Function<UUID, MessageDataHeaderDto> callback) {
        this.onGetRootOfTrust = callback;
        return this;
    }

    /**
     * Callback thats invoked when the validator needs to lookup another record
     */
    public TrustValidatorBuilder withGetPublicKeyCallback(Function<String, MessagePublicKeyDto> callback) {
        this.onGetPublicKey = callback;
        return this;
    }

    /**
     * Builds the validator using the supplied data and a test subject
     */
    public Validator build(IPartitionKey partitionKey, MessageDataDto data) {
        return new Validator(partitionKey, data);
    }

    /**
     * @return Returns true if the validator can be successfully instantiated and all the validation rules executed
     */
    public boolean validate(IPartitionKey partitionKey, MessageDataDto data) {
        try {
            Validator validator = build(partitionKey, data);
            return validator
                    .upgradeWithParentChecks()
                    .upgradeWithLeafState()
                    .validateAll();
        } catch (Throwable ex) {
            failure(data, ex.getMessage());
            return false;
        }
    }

    /**
     * Class returned when a failure occurs
     */
    public final class Failure {
        public final @Nullable LoggerHook LOG;
        public final MessageDataDto data;
        public final String why;

        protected Failure(@Nullable LoggerHook LOG, MessageDataDto data, String why) {
            this.LOG = LOG;
            this.data = data;
            this.why = why;
        }
    }

    /**
     * Loads a data object of interest to this validator
     */
    protected DataContainer getData(UUID id, boolean flush) {
        if (onGetData != null) {
            return onGetData.apply(id, flush);
        } else {
            throw new RuntimeException("Validator attempted to load a data but no callback was supplied to load one.");
        }
    }

    /**
     * Loads a data object of interest to this validator
     */
    protected MessageDataHeaderDto getRootOfTrust(UUID id) {
        if (onGetRootOfTrust != null) {
            return onGetRootOfTrust.apply(id);
        } else {
            throw new RuntimeException("Validator attempted to load the root of trust but no callback was supplied to load one.");
        }
    }

    /**
     * Loads a public key of interest to this validator
     */
    protected MessagePublicKeyDto getPublicKey(String hash) {
        if (onGetPublicKey != null) {
            return onGetPublicKey.apply(hash);
        } else {
            throw new RuntimeException("Validator attempted to load a public key but no callback was supplied to load one.");
        }
    }

    /**
     * Callback thats invoked whenever a validation check fails ot pass
     */
    protected void failure(MessageDataDto data, String why) {
        Failure fail = new Failure(LOG, data, why);
        if (onFailure != null) {
            onFailure.accept(fail);
        } else {
            d.genericLogger.warn(why);
        }
    }

    /**
     * Builder class used to create the trust validator
     */
    public class Validator {
        protected final AteDelegate d = AteDelegate.get();
        protected final UUID id;
        protected final IPartitionKey partitionKey;
        protected final @Nullable UUID parentId;
        protected final @Nullable MessageDataDto existing;
        protected final MessageDataDto data;
        protected final MessageDataHeaderDto header;
        protected final MessageDataDigestDto digest;
        protected final String entityType;

        private MessageDataDto _parent = null;
        private boolean validatedParent = false;
        private boolean validatedIsntReparenting = false;

        protected Validator(IPartitionKey partitionKey, MessageDataDto data) {
            MessageDataHeaderDto header = data.getHeader();
            MessageDataDigestDto digest = data.getDigest();
            if (header == null || digest == null) {
                throw new RuntimeException("Header or digest is invalid for this data object.");
            }

            this.id = header.getIdOrThrow();
            this.partitionKey = partitionKey;
            this.parentId = header.getParentId();
            this.data = data;
            this.header = header;
            this.digest = data.getDigest();
            this.entityType = header.getPayloadClazzOrThrow();

            if (requestTrust != null && requestTrust.containsKey(id)) {
                this.existing = requestTrust.get(id);
            } else {
                DataContainer existingMsg = getData(id, false);
                this.existing = existingMsg != null ? existingMsg.getLastDataOrNull() : null;
            }
        }

        protected Validator(Validator last) {
            this.id = last.id;
            this.partitionKey = last.partitionKey;
            this.parentId = last.parentId;
            this.data = last.data;
            this.header = last.header;
            this.digest = last.digest;
            this.existing = last.existing;
            this.entityType = last.entityType;
            this.validatedParent = last.validatedParent;
        }

        /**
         * Make sure its a valid parent we are attached to (or not)
         */
        public boolean validateParent() {
            if (validatedParent == true) return true;

            @DaoId UUID parentId = header.getParentId();
            MessageDataDto parent = null;
            if (d.daoParents.getAllowedParentsSimple().containsKey(entityType) == false) {
                if (d.daoParents.getAllowedParentFreeSimple().contains(entityType) == false) {
                    fail("parent policy not defined for this entity type");
                    return false;
                }
                if (parentId != null) {
                    fail("parent not allowed for this entity type");
                    return false;
                }
            } else {
                if (parentId == null) {
                    fail("must have parent for this entity type");
                    return false;
                }

                parent = requestTrust != null ? MapTools.getOrNull(requestTrust, parentId) : null;
                if (parent == null) {
                    DataContainer parentMsg = getData(parentId, true);
                    parent = parentMsg != null ? parentMsg.getLastDataOrNull() : null;
                }

                if (parent == null) {
                    fail("parent is missing in chain of trust");
                    return false;
                } else if (d.daoParents.getAllowedParentsSimple().containsEntry(entityType, parent.getHeader().getPayloadClazzOrThrow()) == false) {
                    fail("parent type not allowed [see PermitParentType]");
                    return false;
                }
            }

            this._parent = parent;
            this.validatedParent = true;
            return true;
        }

        /**
         * Now make sure this isnt a duplicate object that has suddenly changed
         * parent ownership (as this would violate the chain of trust)
         */
        public boolean validateIsntReparenting() {
            if (this.validatedIsntReparenting == true) return true;

            if (existing != null) {
                @DaoId UUID existingParentId = existing.getHeader().getParentId();
                if (existingParentId != null && existingParentId.equals(header.getParentId()) == false)
                {
                    fail("parent has changed [was=" + existingParentId + ", now=" + header.getParentId() + "]");
                    return false;
                }

                // If the existing header is immutable then fail this update
                if (existing.getHeader().getInheritWrite() == false && existing.getHeader().getAllowWrite().isEmpty()) {
                    fail("record is immutable");
                    return false;
                }
            }
            this.validatedIsntReparenting = true;
            return true;
        }

        public boolean validateAll() {
            return validateParent() &&
                   validateIsntReparenting();
        }

        /**
         * Upgrades the validator by performing some parent checks and then returning a new state
         * Note: To avoid an exception being throw check the validateAll method before this one
         * @return
         */
        public ValidatorWithParentState upgradeWithParentChecks() {
            if (validateAll() == false) {
                throw new RuntimeException("Attempted to upgrade validator but its in a failed state");
            }
            return new ValidatorWithParentState(this, this._parent);
        }

        /**
         * Callback thats invoked whenever a validation check fails ot pass
         */
        protected void fail(String why) {
            failure(data, why);
        }
    }

    /**
     * Trust validator enhanced with extra information about the parent
     */
    public class ValidatorWithParentState extends Validator {
        protected final @Nullable MessageDataDto parent;

        private @Nullable MessageDataDto _leaf;
        private @Nullable MessagePublicKeyDto _digestPublicKey;
        private boolean _roleFound;
        private boolean validatedLeaf = false;

        protected ValidatorWithParentState(Validator last, @Nullable MessageDataDto parent) {
            super(last);
            this.parent = parent;
        }

        protected ValidatorWithParentState(ValidatorWithParentState last) {
            super(last);
            this.parent = last.parent;
            this.validatedLeaf = last.validatedLeaf;
        }

        /**
         * Get the end of the chain of trust that we will traverse up in order
         * to validate the chain of trust. All writes must have a leaf to follow
         * in order to be saved
         */
        public boolean validateLeaf() {
            if (validatedLeaf) return true;

            MessagePublicKeyDto digestPublicKey = null;
            MessageDataDto leaf = existing;
            if (leaf == null) leaf = parent;
            if (leaf == null)
            {
                String implicitAuthority = d.daoParents.getAllowedImplicitAuthoritySimple().getOrDefault(entityType, null);
                if (implicitAuthority == null) {
                    if (d.daoParents.getAllowedDynamicImplicitAuthoritySimple().containsKey(entityType)) {
                        implicitAuthority = header.getImplicitAuthority().stream().findFirst().orElse(null);
                        if (implicitAuthority == null) {
                            fail("record missing implicit authority");
                            return false;
                        }
                    }
                }

                // If the object is a claimable type then its allowed to attach to nothing
                if (d.daoParents.getAllowedParentFreeSimple().contains(entityType) == true &&
                        d.daoParents.getAllowedParentClaimableSimple().contains(entityType) == true) {
                    MessagePublicKeyDto trustPublicKey = d.encryptor.getTrustOfPublicWrite();
                    digestPublicKey = trustPublicKey;
                    LOG.info("[" + partitionKey + "] chain-of-trust claimed: " + entityType + ":" + id);
                }
                // If the object is a claimable type then its allowed to attach to nothing
                else if (d.daoParents.getAllowedParentFreeSimple().contains(entityType) == true &&
                        implicitAuthority != null)
                {
                    try {
                        MessagePublicKeyDto trustImplicit = d.implicitSecurity.enquireDomainKey(implicitAuthority, true, partitionKey);
                        if (trustImplicit == null) {
                            fail("dns or log record for implicit authority missing [" + implicitAuthority + "]");
                            return false;
                        }

                        digestPublicKey = trustImplicit;
                        LOG.info("[" + partitionKey + "] chain-of-trust rooted: " + entityType + ":" + id + " on " + trustImplicit.getPublicKeyHash());
                    } catch (Throwable ex) {
                        fail(ex.getMessage());
                        return false;
                    }
                }
                // Otherwise we fail
                else {
                    fail("record has no leaf to attach to");
                    return false;
                }
            }

            List<String> availableWriteRoles = new ArrayList<>();
            boolean roleFound = false;
            for (;leaf != null;)
            {
                MessageDataHeaderDto leafHeader = leaf.getHeader();
                Set<String> requiredRoles = leafHeader.getAllowWrite();

                for (String trustKeyHash : requiredRoles) {
                    availableWriteRoles.add(trustKeyHash);
                    if (trustKeyHash.equals(digest.getPublicKeyHash()) == true) {
                        roleFound = true;

                        MessagePublicKeyDto trustPublicKey = getPublicKey(trustKeyHash);
                        if (trustPublicKey != null) digestPublicKey = trustPublicKey;
                        if (digestPublicKey != null) break;
                    }
                }
                if (leafHeader.getInheritWrite() == false) break;

                @DaoId UUID leafParentId = leafHeader.getParentId();
                if (leafParentId != null) {

                    if (requestTrust != null && requestTrust.containsKey(leafParentId)) {
                        leaf = requestTrust.get(leafParentId);
                    } else {
                        DataContainer leafMsg = getData(leafParentId, true);
                        leaf = leafMsg != null ? leafMsg.getLastDataOrNull() : null;
                    }
                } else {
                    leaf = null;
                }
            }
            if (digestPublicKey == null) {
                MessageDataHeaderDto root = getRootOfTrust(id);
                if (root != null) {
                    for (String trustKeyHash : root.getAllowWrite()) {
                        availableWriteRoles.add(trustKeyHash);
                        if (trustKeyHash.equals(digest.getPublicKeyHash()) == true) {
                            roleFound = true;

                            MessagePublicKeyDto trustPublicKey = getPublicKey(trustKeyHash);
                            if (trustPublicKey != null) digestPublicKey = trustPublicKey;
                            if (digestPublicKey != null) break;
                        }
                    }
                }
            }

            if (digestPublicKey == null) {
                noDigest(availableWriteRoles);
                return false;
            }

            this._digestPublicKey = digestPublicKey;
            this._leaf = leaf;
            this._roleFound = roleFound;
            this.validatedLeaf = true;
            return true;
        }

        /**
         * Called when there is no digest present or it could not be found
         */
        private void noDigest(List<String> availableWriteRoles) {
            MessageDataDto leaf = existing;

            if (this._roleFound == true) {
                fail("entity has write roles but public key is missing");
            } else {
                String entityTxt = "clazz=" + entityType + ", id=" + id;

                String parentTxt = "null";
                if (parent != null) { parentTxt = "clazz=" + parent.getHeader().getPayloadClazzOrThrow() + ", id=" + parentId; }

                StringBuilder sb = new StringBuilder();
                sb.append("entity has no right to attach to its parent");
                sb.append("\n [entity: ").append(entityTxt).append("]");
                sb.append("\n [parent: ").append(parentTxt).append("]");
                for (String role : availableWriteRoles) {
                    sb.append("\n [needs: hash=").append(role);
                    MessagePublicKeyDto roleKey = getPublicKey(role);
                    if (roleKey != null && roleKey.getAlias() != null) {
                        sb.append(", alias=").append(roleKey.getAlias());
                    }
                    sb.append("]");
                }
                if (availableWriteRoles.size() <= 0) {
                    if (existing != null) {
                        sb.append("\n [needs: impossible as record is missed write roles.]");
                    } else if (parent != null) {
                        sb.append("\n [needs: impossible as no record or parents exist.]");
                    } else {
                        sb.append("\n [needs: impossible as no record exists and its orphaned.]");
                    }
                }

                sb.append("\n [digest: hash=").append(digest.getPublicKeyHash());
                MessagePublicKeyDto digestKey = getPublicKey(digest.getPublicKeyHash());
                if (digestKey != null && digestKey.getAlias() != null) {
                    sb.append(", alias=").append(digestKey.getAlias());
                }
                sb.append("]");

                sb.append("\n from ");
                fail(sb.toString());
            }
        }

        @Override
        public boolean validateAll() {
            return super.validateAll() &&
                    validateLeaf();
        }

        /**
         * Upgrades the validator by performing some checks on the leaf that the data is attempting to attach to
         * Note: To avoid an exception being throw check the validateAll method before this one
         */
        public ValidatorWithLeaf upgradeWithLeafState() {
            if (validateAll() == false) {
                throw new RuntimeException("Attempted to upgrade validator but its in a failed state");
            }
            return new ValidatorWithLeaf(this, this._leaf, this._digestPublicKey, this._roleFound);
        }
    }

    /**
     * Trust validator enhanced with information about the parent
     */
    public class ValidatorWithLeaf extends ValidatorWithParentState {
        protected final @Nullable MessageDataDto leaf;
        protected final @Nullable MessagePublicKeyDto digestPublicKey;
        protected final boolean roleFound;

        private boolean validatedSignature = false;

        protected ValidatorWithLeaf(ValidatorWithParentState last, @Nullable MessageDataDto leaf, @Nullable MessagePublicKeyDto digestPublicKey, boolean roleFound) {
            super(last);
            this.leaf = leaf;
            this.digestPublicKey = digestPublicKey;
            this.roleFound = roleFound;
        }

        protected ValidatorWithLeaf(ValidatorWithLeaf other) {
            super(other);
            this.leaf = other.leaf;
            this.digestPublicKey = other.digestPublicKey;
            this.roleFound = other.roleFound;
            this.validatedSignature = other.validatedSignature;
        }

        /**
         * Validates that the signature attached to the data object is correct
         */
        public boolean validateSignature() {
            if (validatedSignature == true) return true;

            // Compute the byte representation of the header
            ByteArrayOutputStream stream = new ByteArrayOutputStream();
            MessageSerializer.writeBytes(stream, header.createFlatBuffer());

            // Add the payload itself into the stream
            if (data.hasPayload()) {
                try {
                    byte[] payloadBytes = data.getPayloadBytes();
                    if (payloadBytes != null) {
                        stream.write(payloadBytes);
                    } else {
                        fail("message data has payload but it did not appear to be attached");
                        return false;
                    }
                } catch (IOException ex) {
                    String msg = ex.getMessage();
                    if (msg == null) msg = ex.toString();
                    fail(msg.toLowerCase());
                    return false;
                }
            }
            // Compute the digest bytes
            byte[] streamBytes = stream.toByteArray();
            byte[] seedBytes = digest.getSeedBytes();
            byte[] digestBytes = d.encryptor.hashSha(seedBytes, streamBytes);

            // Verify the digest bytes match the signature
            byte[] digestBytesHeader = digest.getDigestBytes();
            if (Arrays.areEqual(digestBytesHeader, digestBytes) == false) {
                fail("digest differential");
                return false;
            }

            // Now check that the public yields the same digit thus proving that
            // the owner of the private key generated this data
            byte[] sigBytes = digest.getSignatureBytes();

            // Validate that the byte arrays are big enough
            if (digestBytes.length <= 4) {
                fail("digest of payload bytes invalid");
                return false;
            }
            if (sigBytes.length <= 4) {
                fail("signature bytes invalid");
                return false;
            }

            //SLOG.info("ntru-decrypt:\n" + "  - public-key: " + digest.getPublicKey() + "\n  - data: " + digest.getSignature() + "\n");
            if (d.encryptor.verify(digestPublicKey, digestBytes, sigBytes) == false)
            {
                fail("signature verification failed");
                return false;
            }

            this.validatedSignature = true;
            return true;
        }

        @Override
        public boolean validateAll() {
            return super.validateAll() &&
                   validateSignature();
        }
    }
}
