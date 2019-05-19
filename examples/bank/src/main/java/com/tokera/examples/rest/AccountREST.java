package com.tokera.examples.rest;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.Company;
import com.tokera.examples.dao.Individual;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.Consumes;
import javax.ws.rs.POST;
import javax.ws.rs.Path;
import javax.ws.rs.Produces;
import javax.ws.rs.core.MediaType;

@ApplicationScoped
@Path("/account")
public class AccountREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("company")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public PUUID registerCompany(String domain) {
        Account acc = new Account("Company account for " + domain);
        Company company = new Company(domain, acc);
        d.headIO.mergeLater(company);
        acc.company = company.getId();
        d.headIO.mergeLater(acc);

        return company.addressableId();
    }

    @POST
    @Path("individual")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public PUUID registerIndividual(String email) {
        Account acc = new Account("Individual account for " + email);
        Individual individual = new Individual(email, acc);
        d.headIO.mergeLater(individual);
        acc.individual = individual.getId();
        d.headIO.mergeLater(acc);

        return individual.addressableId();
    }
}
