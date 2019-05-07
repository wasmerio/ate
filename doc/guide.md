ATE library reference guide
===========================

## Navigation

- [Executive Summary](../README.md)
- [User Guide for ATE](guide.md)
- [Technical Design of ATE](design.md)
- [Component Design of ATE](components.md)

## Table of Contents

1. [Maven](#maven) 
2. [Bootstrap Application](#bootstrap-application)
3. [Data Access Objects](#data-access-objects)

## Maven

An example maven POM.xml file is described below that will bring in everything thats needs to run the
ATE database along with Undertow, Weld, Kafka, ZooKeeper and their depedencies.

    <?xml version="1.0" encoding="UTF-8"?>
    <project xmlns="http://maven.apache.org/POM/4.0.0"
             xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
             xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
        <modelVersion>4.0.0</modelVersion>
        <groupId>com.tokera</groupId>
        <artifactId>hello-world</artifactId>
        <version>1.0-SNAPSHOT</version>
        <parent>
            <groupId>com.tokera</groupId>
            <artifactId>ate-deps</artifactId>
            <version>0.1.42</version>
        </parent>
        <licenses>
            <license>
                <name>Apache License, Version 2.0</name>
                <url>http://www.apache.org/licenses/LICENSE-2.0.txt</url>
                <distribution>repo</distribution>
            </license>
        </licenses>
        <properties>
            <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
            <maven.compiler.source>1.8</maven.compiler.source>
            <maven.compiler.target>1.8</maven.compiler.target>
            <log4j.version>1.2.17</log4j.version>
            <slf4j.version>1.7.26</slf4j.version>
            <weld.junit.version>1.3.1.Final</weld.junit.version>
            <surefire.version>3.0.0-M3</surefire.version>
            <tokera.ate.version>0.1.42</tokera.ate.version>
        </properties>
        <dependency>
            <groupId>org.jboss.weld</groupId>
            <artifactId>weld-junit5</artifactId>
            <version>${weld.junit.version}</version>
        </dependency>
        <dependency>
            <groupId>org.slf4j</groupId>
            <artifactId>slf4j-api</artifactId>
            <version>${slf4j.version}</version>
        </dependency>
        <dependency>
            <groupId>org.slf4j</groupId>
            <artifactId>slf4j-simple</artifactId>
            <version>${slf4j.version}</version>
            <scope>test</scope>
        </dependency>
        <dependency>
            <groupId>org.slf4j</groupId>
            <artifactId>slf4j-log4j12</artifactId>
            <version>${slf4j.version}</version>
            <scope>runtime</scope>
        </dependency>
        <dependency>
            <groupId>log4j</groupId>
            <artifactId>log4j</artifactId>
            <version>${log4j.version}</version>
        </dependency>
        <dependency>
            <groupId>com.tokera</groupId>
            <artifactId>ate</artifactId>
            <version>${tokera.ate.version}</version>
            <type>jar</type>
        </dependency>
        <build>
            <plugins>
                <plugin>
                    <groupId>org.apache.maven.plugins</groupId>
                    <artifactId>maven-surefire-plugin</artifactId>
                    <version>${surefire.version}</version>
                    <configuration>
                        <classpathDependencyExcludes>
                            <classpathDependencyExcludes>ch.qos.logback:logback-classic</classpathDependencyExcludes>
                            <classpathDependencyExcludes>org.slf4j:slf4j-log4j12</classpathDependencyExcludes>
                        </classpathDependencyExcludes>
                    </configuration>
                </plugin>
                <plugin>
                    <groupId>org.apache.maven.plugins</groupId>
                    <artifactId>maven-compiler-plugin</artifactId>
                    <version>3.8.0</version>
                    <configuration>
                        <compilerArguments>
                            <Xmaxerrs>10000</Xmaxerrs>
                            <Xmaxwarns>10000</Xmaxwarns>
                        </compilerArguments>
                        <fork>true</fork>
                        <source>1.8</source>
                        <target>1.8</target>
                        <compilerArgs>
                            <arg>-XDignore.symbol.file</arg>
                            <arg>-Werror</arg>
                            <arg>-Awarns</arg>
                            <arg>-Alint=all</arg>
                        </compilerArgs>
                    </configuration>
                </plugin>
                <plugin>
                    <groupId>org.apache.maven.plugins</groupId>
                    <artifactId>maven-dependency-plugin</artifactId>
                    <version>3.1.1</version>
                    <executions>
                        <execution>
                            <id>copy-dependencies</id>
                            <phase>prepare-package</phase>
                            <goals>
                                <goal>copy-dependencies</goal>
                            </goals>
                            <configuration>
                                <outputDirectory>${project.build.directory}/lib</outputDirectory>
                                <overWriteReleases>false</overWriteReleases>
                                <overWriteSnapshots>false</overWriteSnapshots>
                                <overWriteIfNewer>true</overWriteIfNewer>
                            </configuration>
                        </execution>
                    </executions>
                </plugin>
                <plugin>
                    <groupId>org.apache.maven.plugins</groupId>
                    <artifactId>maven-jar-plugin</artifactId>
                    <version>3.1.1</version>
                    <configuration>
                        <archive>
                            <index>true</index>
                            <manifest>
                                <addClasspath>true</addClasspath>
                                <mainClass>com.tokera.examples.HelloWorldApp</mainClass>
                                <classpathPrefix>lib</classpathPrefix>
                            </manifest>
                        </archive>
                    </configuration>
                </plugin>
             </plugins>
        </build>
    </project>

The parent POM file is a quicker and easier way to maintain and stay up to date with all the dependencies
required to get a fully operational application up and running. If you want to use a different
application server and/or manage your dependencies independently of the ATE library maintainers then
you can skip this step and add the extra dependencies seperately.

This maven parent will bring in the following dependencies that are needed for ATE to run out-of-the-box

- Weld
- Undertow
- Resteasy
- Kafka
- ZooKeeper 

## Bootstrap Application

The ATE library comes with a bunch of integration and boostraping classes that allow you to get up
and running in the quickest time possible. You can also use these classes to build your own
integration with other application framework however this is out of scope for this guide.

This guide will assume that you are using the bootstrap application and its helper libraries.

    @ApplicationPath("1-0")
    public class HelloWorldApp extends BootstrapApp {
    
        public HelloWorldApp() { }
    
        public static void main(String[] args) {
            start();
        }
    
        public static void start() {
            BootstrapConfig config = ApiServer.startWeld();
            config.setApplicationClass(MainApp.class);
            config.setDeploymentName("Example API");
            config.setRestApiPath("/rs");
            config.setPropertiesFile("example.configuration");
            config.setDomain("examples.tokera.com");
            config.setPingCheckOnStart(true);
            ApiServer apiServer = ApiServer.startApiServer(config);
        }
    }

The bootstrap application will start up Resteasy using Weld's dependency injection engine and also
it will optionally start Kafka and ZooKeeper if the running machine machines the DNS A records. If
you wish to prevent the Kafka and ZooKeeper services from running (for instance in unit tests) then
you can use the following commands:

    ApiServer.setPreventZooKeeper(true);
    ApiServer.setPreventKafka(true);

It is important that you setup the bootstrap configuration class with the right settings for your
particular use case. There is one particularily important settings below

    config.setDomain("examples.tokera.com");

This domain name determines which DNS records to use when seeding the chain-of-trust that allows
records to be accepted into the database. It is highly recommended that you host DNSSec records for
anything used by this library for security reasons.

If you would like to read more about the design of ATE and how important the chain of trust is for
Authentication, Authorization and Integrity then read this guide below:

[Technical Design of ATE](design.md)

The most important things to remember are the following:

- Define a tokdata.{domain} A record that lists the IP addresses of servers that will run the Kafka data  
  logs - this Kafka log holds all the actual data thats ever been recorded into the ATE database and  
  ensures all the records remain in sync.
- Define a tokkeep.{domain} A record that lists the IP addresses of servers that will run the Zookeeper  
  state management service - this dependency is required to ensure the Kafka service runs correctly.  
  In future it would be great to remove the need for this depnedency.
- Define a tokauth.{topic} TXT record and add master public keys for any seeding records that you want  
  your users to be able to write to their particular chain.

Note: All the prefixes defined above can be overridden with calls to the BootstrapConfig class.

Rename the ApplicationPath attribute value and call the BootstrapConfig.setRestApiPath in order to
change the default REST api base URL.

For example to make your API respond to https://api.{domain}/api/1/ then set the following:

- Add a DNS A record for "api" under your domain and point it to your ATE servers.
- Add the following annotation - ApplicationPath("1")
- Call the following method - BoostrapConfig.setRestApiPath("api")

## Data Access Objects

Data access objects are what you create to model out your information domain into strongly typed objects.
The trade off between portability, backwards compatibility and serialization performance all data objects
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

The following annotations and overrides are mandatory for a data object to be recognised by the ATE:

- Dependent - this annotation is required so that Weld can find the class during the bootstrap.
- PermitParentFree or PermitParentType - one of these two annotations tells the chain of trust where  
  it starts and the allowed structure that the tree can take.
- BaseDao.getId() - method must be implemented that returns a UUID that uniquely identifies the instance  
  of the data object within the database.
- BaseDao.getParentId() - method must be implemented that returns the ID of the parent object that this
  particular object is attached to, this must correctly match the PermitParentType annotation.
