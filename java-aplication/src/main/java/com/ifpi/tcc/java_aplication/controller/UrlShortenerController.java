package com.ifpi.tcc.java_aplication.controller;

import com.ifpi.tcc.java_aplication.model.ShortenRequest;
import com.ifpi.tcc.java_aplication.model.ShortenResponse;
import com.ifpi.tcc.java_aplication.service.UrlShortenerService;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.net.URI;

@RestController
public class UrlShortenerController {

    private final UrlShortenerService service;

    public UrlShortenerController(UrlShortenerService service) {
        this.service = service;
    }

    @PostMapping("/shorten")
    public ResponseEntity<ShortenResponse> shorten(@RequestBody ShortenRequest request) {
        String shortUrl = service.shorten(request.url());
        
        return ResponseEntity.status(HttpStatus.CREATED).body(new ShortenResponse(shortUrl));
    }

    @GetMapping("/{code}")
    public ResponseEntity<Void> redirect(@PathVariable String code) {
        return service.resolve(code)
            .map(original -> ResponseEntity
                .status(HttpStatus.FOUND)
                .location(URI.create(original))
                .<Void>build())
            .orElse(ResponseEntity.notFound().build());
    }
}
