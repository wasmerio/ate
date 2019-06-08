package com.tokera.examples.rest;

import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.security.TokenBuilder;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.Company;
import com.tokera.examples.dao.Individual;
import com.tokera.examples.dto.RegistrationResponse;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.context.RequestScoped;
import javax.ws.rs.Consumes;
import javax.ws.rs.POST;
import javax.ws.rs.Path;
import javax.ws.rs.Produces;
import javax.ws.rs.core.MediaType;

@RequestScoped
@Path("/register")
public class RegisterREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("/company")
    @Produces(MediaType.APPLICATION_JSON)
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public RegistrationResponse registerCompany(String domain) {
        Account acc = new Account("Company account for " + domain);

        Company company = new Company(domain, acc);
        acc.company = company.getId();

        // Create access rights and grant them to ourselves
        d.authorization.authorizeEntity(company, company);
        d.currentRights.impersonate(company);
        d.io.mergeLater(company);

        // Now save the account using this access rights
        d.io.mergeLater(acc);

        TokenDto token = new TokenBuilder()
                .withUsername("root@" + company.domain)
                .withCompanyName(company.domain)
                .withUserRole(UserRole.HUMAN)
                .withRiskRole(RiskRole.HIGH)
                .withPartitionkeyFromDao(company)
                .addReadKey(d.authorization.getOrCreateImplicitRightToRead(company))
                .addWriteKey(d.authorization.getOrCreateImplicitRightToWrite(company))
                .build();
        return new RegistrationResponse(company.getId(), company.companyAccount, token);
    }

    @POST
    @Path("/individual")
    @Produces(MediaType.APPLICATION_JSON)
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public RegistrationResponse registerIndividual(String email) {
        Account acc = new Account("Individual account for " + email);
        Individual individual = new Individual(email, acc);
        acc.individual = individual.getId();

        // Create access rights and grant them to ourselves
        d.authorization.authorizeEntity(individual, individual);
        d.currentRights.impersonate(individual);
        d.io.mergeLater(individual);

        // Now save the account using this access rights
        d.io.mergeLater(acc);

        TokenDto token = new TokenBuilder()
                .withUsername(individual.email)
                .withUserRole(UserRole.HUMAN)
                .withRiskRole(RiskRole.HIGH)
                .withPartitionkeyFromDao(individual)
                .addReadKey(d.authorization.getOrCreateImplicitRightToRead(individual))
                .addWriteKey(d.authorization.getOrCreateImplicitRightToWrite(individual))
                .build();
        return new RegistrationResponse(individual.getId(), individual.personalAccount, token);
    }
}
