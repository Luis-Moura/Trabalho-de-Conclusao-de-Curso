import http from "k6/http";
import { check } from "k6";
import { randomString } from "https://jslib.k6.io/k6-utils/1.2.0/index.js";

export const options = {
  scenarios: {
    escalabilidade: {
      executor: "ramping-arrival-rate",
      startRate: 10,
      timeUnit: "1s",
      preAllocatedVUs: 500,
      maxVUs: 3000,
      stages: [
        { duration: "3m", target: 1500 },
        { duration: "5m", target: 1500 },
        { duration: "1m", target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_duration: ["p(50)<200", "p(95)<500", "p(99)<1000"],
    http_req_failed: ["rate<0.05"],
  },
};

export default function () {
  const payload = JSON.stringify({
    url: `https://dominio.com/${randomString(15)}`,
  });

  const params = {
    headers: {
      "Content-Type": "application/json",
    },
  };

  // CORREÇÃO 1: Adicionado o '/shorten' no final da URL
  const postRes = http.post("http://172.17.0.1:8080/shorten", payload, params);

  check(postRes, {
    post_status_201: (r) => r.status === 201, // Seu controller retorna HttpStatus.CREATED (201)
  });

  if (postRes.status === 201) {
    let fullShortUrl = "";
    try {
      // CORREÇÃO 2: Pega o campo correto do seu DTO (presumindo que seja 'shortUrl')
      // Se o seu record for `public record ShortenResponse(String url)`, mude aqui para 'url'.
      fullShortUrl = postRes.json("shortUrl");
    } catch (e) {
      console.error("Falha ao ler o JSON:", e);
    }

    if (fullShortUrl) {
      // Extrai apenas o código da URL completa gerada pelo Java
      const urlParts = fullShortUrl.split("/");
      const code = urlParts[urlParts.length - 1];

      // Faz o GET passando apenas o código
      const getRes = http.get(`http://172.17.0.1:8080/${code}`, {
        redirects: 0,
      });

      check(getRes, {
        // O seu controller retorna HttpStatus.FOUND, que é o código 302
        get_status_302: (r) => r.status === 302,
      });
    }
  }
}
