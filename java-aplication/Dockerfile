# ─── Build stage ────────────────────────────────────────────────────────────
FROM eclipse-temurin:21-jdk-alpine AS build

WORKDIR /app
COPY .mvn/ .mvn/
COPY mvnw pom.xml ./
RUN ./mvnw dependency:go-offline -B
COPY src/ src/
RUN ./mvnw package -DskipTests -B

# ─── Runtime stage ──────────────────────────────────────────────────────────
FROM eclipse-temurin:21-jre-alpine AS runtime

WORKDIR /app
RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

COPY --from=build /app/target/*.jar app.jar

# MaxRAMPercentage=50%: em 512MB = ~256MB de heap
# Deixa ~256MB para Metaspace, code cache, buffers NIO do Tomcat e SO
ENV JAVA_TOOL_OPTIONS="-XX:MaxRAMPercentage=50.0 -XX:InitialRAMPercentage=25.0"

EXPOSE 8080
ENTRYPOINT ["java", "-jar", "app.jar"]
