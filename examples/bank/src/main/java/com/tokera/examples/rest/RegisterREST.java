package com.tokera.examples.rest;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.security.TokenBuilder;
import com.tokera.ate.security.TokenSecurity;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.Company;
import com.tokera.examples.dao.Individual;
import com.tokera.examples.dto.RootLoginRequest;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.Consumes;
import javax.ws.rs.POST;
import javax.ws.rs.Path;
import javax.ws.rs.Produces;
import javax.ws.rs.core.MediaType;

@ApplicationScoped
@Path("/register")
public class RegisterREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("/company")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public Company registerCompany(String domain) {
        Account acc = new Account("Company account for " + domain);
        Company company = new Company(domain, acc);
        acc.company = company.getId();

        // Create access rights and grant them to ourselves
        d.authorization.authorizeEntity(company, company);
        d.currentRights.impersonate(company);
        d.headIO.mergeLater(company);

        // Now save the account using this access rights
        d.headIO.mergeLater(acc);
        return company;
    }

    @POST
    @Path("/individual")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public Individual registerIndividual(String email) {
        Account acc = new Account("Individual account for " + email);
        Individual individual = new Individual(email, acc);
        acc.individual = individual.getId();

        // Create access rights and grant them to ourselves
        d.authorization.authorizeEntity(individual, individual);
        d.currentRights.impersonate(individual);
        d.headIO.mergeLater(individual);

        // Now save the account using this access rights
        d.headIO.mergeLater(acc);
        return individual;
    }

    @POST
    @Path("/root-login")
    @Produces(MediaType.APPLICATION_XML)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    @PermitAll
    public String rootLogin(RootLoginRequest request) {
        return new TokenBuilder()
                .withUsername(request.getUsername())
                .addReadKeys(request.getReadRights())
                .addWriteKeys(request.getWriteRights())
                .shouldPublish(true)
                .build()
                .getXmlToken();
    }
}
