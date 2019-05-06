Technical Design for ATE
========================

## Navigation

- [Executive Summary](../README.md)
- [User Guide for ATE](guide.md)
- [Technical Design of ATE](design.md)

## Table of Contents

1. [Immutable Data](#immutable-data)
2. [Distributed Architecture](#distributed-architecture)
3. [Share Nothing](#share-nothing)
4. [Absolute Portability](#absolute-portability)
5. [Chain of Trust](#chain-of-trust)
6. [Implicit Authority](#implicit-authority)
7. [Fine Grained Security](#fine-grained-security)
9. [Quantum Resistent](#quantum-resistent)
10. [Eventually Consistent Caching](#eventually-consistent-caching)
11. [Native REST Integrated](#native-rest-integrated)
12. [Component Design](#component-design)

## Immutable Data

## Distributed Architecture

## Share Nothing

## Absolute Portability

## Chain Of Trust

## Implicit Authority

## Fine Grained Security

## Quantum Resistent

## Eventually Consistent Caching

## Undertow and Weld

## Native REST Integrated

## Component Design

### com.tokera.ate.annotations

Declares the custom annotations used by ATE, one design goal with the annotations of ATE was to
minimize the number of new annotations when external but existing ones already exist thus the majority
of annotations defined are for its unique features, in particular, its advanced authentication and
authorization engine.

### com.tokera.ate.client

REST client class and helpers that make it easier to call Resteasy supporting ATE processes with less
boilerplate coding which is especially useful for unit tests.

### com.tokera.ate.common

Contains all the odd classes that dont easily fit into a generic categorization. Some notable classes
here are:

- Immutalizable object containers that operate like a normal container but can be marked as immutable at any time.
- LoggerHook that makes it easier to add context aware loggers
- String, Xml, Uuid and Uri helper classes

### com.tokera.ate.configuration

Currently this namespace (package) just holds a bunch of constants you shouldnt really need to touch.

### com.tokera.ate.constaints

Custom validations for data fields and types defined for ATE - in particular - the private and public
key types have validators here.

### com.tokera.ate.dao

Holds all the base classes, interfaces and message flatbuffer serializers that underpin the data objects
stored and retrieved from the ATE database.

### com.tokera.ate.delegates

Core functionality is split into seperate delegate worker classes that perform specific roles and
responsibilities within ATE. Grouping and seperating the functional logic makes everything else cleaner,
improves the readability of the code and reduces bugs by better managing its quality and complexity.

### com.tokera.ate.dto

Classes under this package are "Data Transfer Objects" which used for transferring explicit strongly
typed data over the wire between instances and APIs. Some important objects reside here such as the
message encapsulation classes that have built in COW (copy-on-write) sementics and the token classes
that hold the SAML XML documents.

### com.tokera.ate.enumerations

As implied by the name of this package it holds all the custom defined enumerations used by this library.

### com.tokera.ate.events

ATE uses the events architecture of the depedency injection frameworks (in this case Weld) in order
to notify impacted beans of certain critical events. These include (but are not limited to):

- Events triggered when the Token current in effect changes.
- Events triggered when the access rights of the current context change.
- Events triggered when a newly loaded Topic needs to be seeded with the root of the chain-of-trust.

### com.tokera.ate.exceptions

Some minor helper classes here are used to bundle up validation violations.

### com.tokera.ate.extensions

ATE defined extensions here effectively plugin and hook up key parts of the ATE engine so that it needs
minimum boilerplate code and can self-discover and configure itself - notable extensions include:

- DaoParentDiscoveryExtension which parses all the Data Objects and builds a tree of allowed parent
  child relationships.
- ResourceScopedExtension adds a new scope named ResourceScoped that is unique for every API method
  defined on Resteasy APIs. This is useful for caching reflection calls and pre-loading the authority
  rules that are defined as method annotations.
- SerializableObjectExtension holds the main serializer that turns DAO's (Data Access Objects) into
  byte streams that encrypted and then stored on the distributed commit logs.
- StartupBeanExtension ensures that any ApplicationScoped bean that is also marked with the Startup
  annotation will automatically load and call any PostConstruct methods during the startup and
  initialization phase of the application.
- TokenScopedExtension adds a new scope named TokenScoped that is unqiue for any token that is used
  on the Resteasy API. Token scopes allow the parsed and heavily lifting of processing tokens to be
  done once instead of on every API call while still maintaining the high level of security that
  immutable tokens with fine grained claims brings.
- YamlTagDiscoveryExtension allows serializable data transfer objects (DTOs) that are marked with the
  YamlTag annotation to use a short hand tag rather than the fully qualified class name during
  serialization calls.

### com.tokera.ate.filters

Filters are relevant only for the Resteasy pipeline and effectively hook into the flow of actions
performed on API calls as they are processed - notable filters include:

- AccessLogInterceptor will pass headers back to the caller that describe which data objects were
  modified in a particular API call (Invalidate) and which data objects were used in coming up with
  the response and thus would make the result different if they changed (Track). Effectively these
  headers allow the caller to build a client side cache of the API call results.
- AuthorityInterceptor will check that the caller is allowed to invoke a particular API method by
  checking that the supplied token (Authorization header) includes the needed claims (Permit annotations)
- CorsInterceptor adds the Cors headers into the responses so that Cors functionality is properly
  handled by browsers.
- DefaultBootstrapInit ensures that the OpenSAML library is initialized before it attempts to do anything.
- ExceptionInterceptor will turn RuntimeExceptions into HTTP errors that give callers better error
  handling logic and allow for an easier bug fixing experience.
- FixResteasyBug fixes a bug in Resteasy where attempts to use parameter maps does not work if they
  are not first accessed in an interceptor.
- LogReferenceInterceptor and ReferenceIdInterceptor combined allow for a collelation ID to be passed
  between multiple API calls and hence they make it easier to track and debug complex call trees that
  cross REST call boundaries.
- ResourceScopeInterceptor will start the ResourceScope for any API methods that are invoked.
- ResourceStatsInterceptor performs atomic counts on methods so that heavily loaded methods can take
  proactive or reactive measures but ultimately it was build to show how a ResourceScope could be used.
- TopicInterceptor allows the caller to specifc a Kafka Topic that the API should execute its logic
  under using the "Topic" Header. If no header is supplied then the ATE library will use the domain
  name of the USERNAME claim written in the supplied token as the topic name.
- TransactionInterceptor will commit any records marked for saving (d.headIO.mergeLater) into the
  chain-of-trust before returning to the caller (but only if the return code is a success as otherwise
  no data changes will be committed)
- VersionInterceptor writes a "Version" header so that can be used to invalidate client side caching
  when version upgrades occur of the API. Otherwise API releases wouldnt take effect after the caches
  and tokens timeout (if at all).

### com.tokera.ate.io

### com.tokera.ate.kafka

### com.tokera.ate.providers

### com.tokera.ate.qualifiers

Qualifiers used by the dependency injection engine to configure and setup the systems that manage the
data.

### com.tokera.ate.scopes

### com.tokera.ate.security

### com.tokera.ate.token

### com.tokera.ate.units

### com.tokera.ate.ApiServer

### com.tokera.ate.BootstrapApp

### com.tokera.ate.BootstrapConfig

### com.tokera.ate.KafkaServer

### com.tokera.ate.ZooServer
