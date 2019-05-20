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
@Path("/register")
public class RegisterREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("company")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public PUUID registerCompany(String domain) {
        Account acc = new Account("Company account for " + domain);
        Company company = new Company(domain, acc);
        acc.company = company.getId();
        d.headIO.mergeLater(company);
        d.headIO.mergeLater(acc);
        return company.addressableId();
    }

    @POST
    @Path("individual")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public PUUID registerIndividual(String email) {
        Account acc = new Account("Individual account for " + email);
        Individual individual = new Individual(email, acc);
        acc.individual = individual.getId();
        d.headIO.mergeLater(individual);
        d.headIO.mergeLater(acc);
        return individual.addressableId();
    }
}
