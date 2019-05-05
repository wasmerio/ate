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
7. [Fine Grained Security)(#fine-grained-security)
9. [Quantum Resistent](#quantum-resistent)
10. [Eventually Consistent Caching](#eventually-consistent-caching)
11. [Native REST Integrated](#native-rest-integrated)
12. [Component Design](#component-design)
    1. [com.tokera.ate.annotations](#com.tokera.ate.annotations)

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

Currently this namespace (package) just holds a bunch of constants you shouldnt really need to touch

### com.tokera.ate.constaints

Custom validations for data fields and types defined for ATE - in particular - the private and public
key types have validators here.

### com.tokera.ate.dao

Holds all the base classes, interfaces and message flatbuffer serializers that underpin the data objects
stored and retrieved from the ATE database.

### com.tokera.ate.delegates

Core functionality is split into seperate delegate worker classes that perform specific roles and
responsibilities within ATE. Grouping and seperating the functional logic makes everything else cleaner,
improves the readability of the code and reduces bugs by better managing its quality.

### com.tokera.ate.dto

### com.tokera.ate.enumerations

### com.tokera.ate.events

### com.tokera.ate.exceptions

### com.tokera.ate.extensions

### com.tokera.ate.filters

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
