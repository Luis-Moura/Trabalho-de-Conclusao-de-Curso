package com.ifpi.tcc.java_aplication.repository;

import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Repository;

import java.util.Optional;

@Repository
public class UrlRepository {

    private final JdbcTemplate jdbc;

    public UrlRepository(JdbcTemplate jdbc) {
        this.jdbc = jdbc;
    }

    public void save(String shortCode, String originalUrl) {
        jdbc.update(
            "INSERT INTO urls (short_code, original_url) VALUES (?, ?)",
            shortCode, originalUrl
        );
    }

    public Optional<String> findOriginalUrlByCode(String shortCode) {
        return jdbc.query(
            "SELECT original_url FROM urls WHERE short_code = ?",
            (rs, rowNum) -> rs.getString("original_url"),
            shortCode
        ).stream().findFirst();
    }
}
