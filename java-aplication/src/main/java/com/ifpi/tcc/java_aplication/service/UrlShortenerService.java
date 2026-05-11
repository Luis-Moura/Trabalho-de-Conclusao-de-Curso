package com.ifpi.tcc.java_aplication.service;

import com.ifpi.tcc.java_aplication.repository.UrlRepository;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.stereotype.Service;

import java.security.SecureRandom;
import java.util.Optional;

@Service
public class UrlShortenerService {

    private static final String BASE62 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    private static final int CODE_LENGTH = 7;
    private static final int MAX_RETRIES = 3;

    private final SecureRandom random = new SecureRandom();
    private final UrlRepository repository;
    private final String baseUrl;

    public UrlShortenerService(UrlRepository repository, @Value("${app.base-url}") String baseUrl) {
        this.repository = repository;
        this.baseUrl = baseUrl;
    }

    public String shorten(String originalUrl) {
        for (int attempt = 0; attempt < MAX_RETRIES; attempt++) {
            String code = generateCode();
            
            try {
                repository.save(code, originalUrl);
                return baseUrl + "/" + code;
            } catch (DataIntegrityViolationException e) {
                // short_code collision — regenerate and retry
            }
        }
        throw new IllegalStateException("Failed to generate a unique short code after " + MAX_RETRIES + " attempts");
    }

    public Optional<String> resolve(String code) {
        return repository.findOriginalUrlByCode(code);
    }

    private String generateCode() {
        byte[] bytes = new byte[CODE_LENGTH];

        random.nextBytes(bytes);

        StringBuilder sb = new StringBuilder(CODE_LENGTH);

        for (byte b : bytes) {
            sb.append(BASE62.charAt(Math.abs(b % BASE62.length())));
        }

        return sb.toString();
    }
}
