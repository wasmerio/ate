ATE Components and Packages
===========================

## Navigation

- [Executive Summary](../README.md)
- [User Guide for ATE](guide.md)
- [Technical Design of ATE](design.md)
- [Component Design of ATE](components.md)

### com.tokera.ate.annotations

Declares the custom annotations used by ATE. A key design goal with the annotations was to minimize the
creation of new custom annotations when perfectly useable annotations already exist elsewhere thus the
majority of annotations defined in this library are for its unique features - in particular - its
advanced authentication and authorization engine - more details are as follows:

- **HideLog**, **VerboseLog**, **ShowLogs** are used to control the level of logging performed on
  different methods which is especially important for sensitive data such as passwords.
- **PermitParentFree** marks DAO (Data Access Objects) that are allowed to be the root of a
   chain-of-trust (as in they do not need to be attached to a parent)
- **PermitParentType** lists all the parent DAOs types that a particular child DAO can be attached to
  in the chain-of-trust
- **PermitReadEntity** marks a particular path parameter that will be validated for authority before the
  method is allowed to be invoked (otherwise an access violation will occur). In this case the supplied
  token must include an authorization claim that matches the path parameter value or it must exist
  programmically in the CurrentRightsDelegte. The specific needed claim must be read right as the ability
  to read and write are two explicitly seperate permissions.
- **PermitWriteEntity** is similar to a **PermitReadEntity** but is for write claims.
- **PermitRiskRole** will restrict methods to a particular risk level, this is useful for classifying
  methods based on the level of security risk they pose and then using this permission to ensure these
  high risk methods can only be invoked by tokens that were generated via a stronger authentication
  method such as multi-factor authentication.
- **PermitUserRole** allows for different methods to be restricted between humans and automation thus
  ensuring that certain operation tasks are restricted. In practice the use of this annotation is rare.
- **YamlTag** allows DTO (Data Transfer Objects) to override the fully qualified naming of YAML objects
  with a shortened version instead.

### com.tokera.ate.client

REST client class and helpers to reduce the amount of boilerplate coding required to call ATE enabled
resteasy APIs, which is especially useful for unit tests.

### com.tokera.ate.common

Contains all the odd classes that dont easily fit into a generic categorization.  
Some notable classes here are:

- Immutalizable object containers that operate like a normal container but can be marked as immutable at any time.
- LoggerHook that makes it easier to add context aware loggers using dependency injection.
- String, Xml, Uuid and Uri helper classes.

### com.tokera.ate.configuration

Currently this package just holds a bunch of constants one shouldnt really need to touch.

### com.tokera.ate.constaints

Custom validations for data fields and types defined for ATE - in particular - the private and public
key types have validators maintained here.

### com.tokera.ate.dao

Holds all the base classes, interfaces and message flatbuffer serializers that underpin the data objects
stored and retrieved from the ATE database.

### com.tokera.ate.delegates

Core functionality is split into separate delegate functional classes that perform specific roles and
responsibilities within ATE. Grouping and separating the functional logic makes the code cleaner,
improves the readabilityand reduces bugs by better managing the overall complexity and hence quality.

### com.tokera.ate.dto

Classes under this package are "Data Transfer Objects" which are used for transfering explicit strongly
typed data over the wire between APIs and their clients. Some important objects reside here such as the
data message encapsulation classes that have built in COW (copy-on-write) sementics and the token classes
that hold the SAML XML documents.

### com.tokera.ate.enumerations

As implied by the name of this package it holds all the custom defined enumerations used by this library.

### com.tokera.ate.events

ATE uses the events architecture of the dependency injection frameworks (in this case Weld) in order
to notify impacted beans of certain critical events.  
These include (but are not limited to):

- Events triggered when the current token changes.
- Events triggered when the access rights of the current context change.
- Events triggered when a newly loaded Topic needs to be seeded with the root of the chain-of-trust.

### com.tokera.ate.exceptions

Some minor helper classes here are used to bundle up validation violations.

### com.tokera.ate.extensions

ATE defined extensions here effectively plugin and hook up key parts of the ATE engine so that it needs
minimum boilerplate code and can self-discover and configure itself - notable extensions include:

- **DaoParentDiscoveryExtension** which parses all the Data Objects and builds a tree of allowed parent
  child relationships.
- **ResourceScopedExtension** adds a new scope named ResourceScoped that is unique for every API method
  defined on Resteasy APIs. This is useful for caching reflection calls and pre-loading the authority
  rules that are defined as method annotations.
- **SerializableObjectExtension** holds the main serializer that turns DAO's (Data Access Objects) into
  byte streams that encrypted and then stored on the distributed commit logs.
- **StartupBeanExtension** ensures that any ApplicationScoped bean that is also marked with the Startup
  annotation will automatically load and call any PostConstruct methods during the startup and
  initialization phase of the application.
- **TokenScopedExtension** adds a new scope named TokenScoped that is unqiue for any token that is used
  on the Resteasy API. Token scopes allow the parsed and heavily lifting of processing tokens to be
  done once instead of on every API call while still maintaining the high level of security that
  immutable tokens with fine grained claims brings.
- **YamlTagDiscoveryExtension** allows serializable data transfer objects (DTOs) that are marked with the
  YamlTag annotation to use a short hand tag rather than the fully qualified class name during
  serialization calls.

### com.tokera.ate.filters

Filters are relevant only for the Resteasy pipeline and effectively hook into the flow of actions
performed on API calls as they are processed - notable filters include:

- **AccessLogInterceptor** will pass headers back to the caller that describe which data objects were
  modified in a particular API call (Invalidate) and which data objects were used in coming up with
  the response and thus would make the result different if they changed (Track). Effectively these
  headers allow the caller to build a client side cache of the API call results.
- **AuthorityInterceptor** will check that the caller is allowed to invoke a particular API method by
  checking that the supplied token (Authorization header) includes the needed claims (Permit annotations)
- **CorsInterceptor** adds the Cors headers into the responses so that Cors functionality is properly
  handled by browsers.
- **DefaultBootstrapInit** ensures that the OpenSAML library is initialized before it attempts to do anything.
- **ExceptionInterceptor** will turn RuntimeExceptions into HTTP errors that give callers better error
  handling logic and allow for an easier bug fixing experience.
- **FixResteasyBug** fixes a bug in Resteasy where attempts to use parameter maps does not work if they
  are not first accessed in an interceptor.
- **LogReferenceInterceptor** and **ReferenceIdInterceptor** combined allow for a collelation ID to be passed
  between multiple API calls and hence they make it easier to track and debug complex call trees that
  cross REST call boundaries.
- **ResourceScopeInterceptor** will start the **ResourceScope** for any API methods that are invoked.
- **ResourceStatsInterceptor** performs atomic counts on methods so that heavily loaded methods can take
  proactive or reactive measures but ultimately it was build to show how a ResourceScope could be used.
- **TopicInterceptor** allows the caller to specifc a Kafka Topic that the API should execute its logic
  under using the "Topic" Header. If no header is supplied then the ATE library will use the domain
  name of the USERNAME claim written in the supplied token as the topic name.
- **TransactionInterceptor** will commit any records marked for saving (d.headIO.mergeLater) into the
  chain-of-trust before returning to the caller (but only if the return code is a success as otherwise
  no data changes will be committed)
- **VersionInterceptor** writes a "Version" header so that can be used to invalidate client side caching
  when version upgrades occur of the API. Otherwise API releases wouldnt take effect after the caches
  and tokens timeout (if at all).

### com.tokera.ate.io

Contains the core classes and backend enginer for the ATE database. This includes the chain-of-trust
validation, DAO transaction merging logic and the StorageSystemFactory that configures the backend for
a particular use-case.

See the [User Guide](guide.md) for details on how to set this up.

### com.tokera.ate.kafka

Holds the database backend that uses the Kafka distributed commit log as its main storage backend.

### com.tokera.ate.providers

Contains a bunch of YamlSerializers for common data types plus Resteasy serializers that allow for
data streaming and native YAML media types.

### com.tokera.ate.qualifiers

Qualifiers used by the dependency injection engine to configure and setup the systems that manage the
data.

### com.tokera.ate.scopes

Contains the custom scopes used by ATE that simplify the complexity - these scopes are:

- ResourceScoped which is unique for each Resteasy method thats invoked
- TokenScoped which is unique for each Token thats passed to the Resteasy call

### com.tokera.ate.security

All the special security classes reside here such including some helper classes for creating and
manipulating tokens but critically the NTRU encryption helpers that allow for strong authentication
and authorization of data records with built in resistance to quantum attacks. Further this includes
a special seeding modification that allows for NTRU key pairs to be generated in a deterministic but
difficult to crack way.

### com.tokera.ate.token

Contains the OpenSAML writing and validation logic.

### com.tokera.ate.units

Bunch of generic unit qualifiers that make it generic types more strongly typed and improve the
richness of the limited java type system. 

### com.tokera.ate.**ApiServer**

Main class to invoke when bootstrapping your application. Alternatively you can configure the dependency
injection sub-system and your application server without this helper class. Regardless this class
shows you how to connect and configure everything.

### com.tokera.ate.**BootstrapApp**

Base application class that you can extend to minimize bootstrapping code on the
javax.ws.rs.core.Application class

### com.tokera.ate.**BootstrapConfig**

Main configuration class to modify when tuning the ATE database engine to your particular use case.

### com.tokera.ate.**KafkaServer**

ApplicationScoped bean that will configure and start the Kafka sub-system within this same JVM with
minimal operational overhead. Preventing this server from starting and instead hosting your own
Kafka instances is also possible.

### _com.tokera.ate._**ZooServer**

ApplicationScoped bean that will configure and start the ZooKeeper sub-system within this same JVM
with minimum operational overhead. Preventing this server from starting and instead hosting your own
ZooKeeper instance is also possible.
