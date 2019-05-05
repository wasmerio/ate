ATE library reference guide
===========================

# Contents

1. [Data Objects](#data-objects)

# Data Objects

Data objects are what you create to model out your information domain into strongly typed objects. The 
trade off between portability, backwards compatibility and serialization performance all data objects
are stored within the ATE database as small encrypted JSON documents.

Below is an example data object

    @Dependent
    @YamlTag("dao.mything")
    @PermitParentType(MyAccount.class)
    public class MyThing extends BaseDao {
        @Column
        public @DaoId UUID id = UUID.randomUUID();
        @Column
        public @DaoId UUID accountId;
    
        @SuppressWarnings("initialization.fields.uninitialized")
        @Deprecated
        public MyThing() {
        }
    
        public MyThing(MyAccount acc) {
            this.accountId = acc.id;
        }
    
        @Override
        public @DaoId UUID getId() {
            return id;
        }
    
        @Override
        public @Nullable @DaoId UUID getParentId() {
            return null;
        }
    }
