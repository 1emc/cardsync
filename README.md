# galcard

galcard ist ein experimenteller Privacy-First GAL-to-CardDAV Dienst in Rust. Der Dienst liest GAL-ähnliche Benutzerinformationen aus Microsoft 365 über Microsoft Graph, cached sie mandantenfähig in PostgreSQL und stellt sie iOS/iPadOS als read-only CardDAV-Adressbuch bereit.

## Was ist galcard?

Der MVP bildet diesen technischen Flow ab:

```text
Microsoft 365 Tenant
    -> Microsoft Graph Full Sync /users
Lokale PostgreSQL-Datenbank
    -> vCard 4.0 Generierung
Read-only CardDAV Endpoint
    -> Basic Auth / App Password
iOS Kontakte App
```

## Status

- MVP / experimental
- read-only CardDAV
- not production ready yet
- kein Admin-Portal, kein Billing, kein LDAP, keine Kontaktfotos

## Architektur

Das Repository ist als Rust Workspace getrennt nach Verantwortlichkeiten aufgebaut:

- `crates/api`: axum HTTP API, Konfiguration, Admin-Endpunkte und CardDAV-Routing.
- `crates/domain`: Mandanten-, Kontakt-, CardDAV-User- und Privacy-Domänenmodelle.
- `crates/graph`: Microsoft Graph Client-Credentials-Auth und Full Sync für `/users`.
- `crates/carddav`: CardDAV-Hilfsfunktionen, Basic Auth, Argon2id-Passwort-Hashing und XML-Antworten.
- `crates/vcard`: vCard-4.0-Builder, Escaping und ETag-Erzeugung.
- `crates/privacy`: E-Mail-Normalisierung, HMAC-SHA256-Suppression und konstante Vergleiche.

## Features in Version 0.1

- `GET /health` ohne Authentifizierung.
- Admin API mit statischem Bearer Token aus `ADMIN_API_TOKEN`.
- Mandanten anlegen und auflisten.
- CardDAV-User anlegen und Passwort rotieren; Klartextpasswort wird nur einmal zurückgegeben.
- Testkontakte anlegen.
- Microsoft Graph Full Sync über Client Credentials, wenn `MICROSOFT_*` gesetzt ist.
- PostgreSQL-Migrationen für Tenants, Connections, Contacts, CardDAV Users, Sync State, Suppression und Audit Log.
- Read-only CardDAV mit `OPTIONS`, `PROPFIND`, `REPORT`, `GET`.
- Blockierung von `PUT`, `DELETE`, `MKCOL`, `PROPPATCH`, `MOVE`, `COPY`, `LOCK`, `UNLOCK` mit `403 Forbidden`.
- Privacy Suppression per HMAC-SHA256 statt Klartext-E-Mail-Speicherung.

## Privacy by Design

- Es werden keine Kontaktfotos synchronisiert.
- Es werden keine E-Mails, Kalender, Teams-Chats, Dateien, OneDrive-Inhalte oder Mailbox-Kontakte gelesen.
- Der Graph Request nutzt `$select` und beschränkt sich auf die im MVP benötigten Felder.
- Mobilnummern können gespeichert werden, werden im MVP aber standardmäßig nicht in vCards ausgegeben.
- Suppressed und gelöschte Kontakte werden nicht über CardDAV ausgeliefert.
- Logs dürfen keine Namen, E-Mail-Adressen, Telefonnummern, Tokens, Authorization Header, Graph Response Bodies oder vCards enthalten.
- Suppression speichert nur HMAC-SHA256 mit serverseitigem Secret/Pepper. Das ist Pseudonymisierung, keine vollständige Anonymisierung.

## Microsoft App Registration und externe Anforderungen

### Entra App Registration für lokale Entwicklung

Für Microsoft 365 Sync wird eine Microsoft Entra App Registration benötigt. Für lokale MVP-Tests wird Client Credentials Flow unterstützt:

1. Entra Admin Center öffnen.
2. App Registration erstellen.
3. Single Tenant oder Multi Tenant je nach Testszenario wählen.
4. Client Secret erzeugen.
5. API Permissions hinzufügen.
6. Application Permission `User.Read.All` hinzufügen.
7. Admin Consent erteilen.
8. Tenant ID, Client ID und Client Secret in `.env` eintragen.

### Graph Permissions

MVP:

```text
User.Read.All
```

Optional später:

```text
Group.Read.All
Contacts.Read
Directory.Read.All
```

- `User.Read.All`: Benutzerprofile lesen; für den MVP ausreichend.
- `Group.Read.All`: Gruppen und Mitgliedschaften für spätere Filter.
- `Contacts.Read`: Outlook/Exchange-Kontakte, falls später relevant.
- `Directory.Read.All`: deutlich weitreichender und sollte vermieden werden, solange `User.Read.All` ausreicht.

### Admin Consent

Application Permissions erfordern Admin Consent. Kunden müssen nachvollziehen können, welche Graph Permissions angefordert werden und welche Daten gelesen werden.

### Multi-Tenant SaaS Hinweise

Für echten SaaS-Betrieb sind erforderlich:

- Multi-Tenant App Registration.
- Admin Consent Flow.
- Redirect URI für ein späteres Admin Portal.
- Publisher Verification empfohlen.
- Transparente Darstellung der angeforderten Graph Permissions.
- Minimal notwendige Berechtigungen.
- Terms of Service und Privacy Policy URLs.
- Verschlüsselte Speicherung von Secrets und Tokens.

### Warum kein EWS?

Für Exchange Online soll EWS nicht als neue technische Basis verwendet werden. Microsoft hat die Deaktivierung von EWS in Exchange Online angekündigt. galcard verwendet für Microsoft 365 Microsoft Graph. Exchange OnPrem kann später separat über LDAP/LDAPS oder einen OnPrem Connector angebunden werden.

### Warum Microsoft Graph?

Microsoft Graph ist die moderne API-Oberfläche für Microsoft 365. Der MVP liest ausschließlich `/users` mit gezieltem `$select` und schreibt keine Daten nach Microsoft 365 zurück.

### Rate Limits und Delta Sync

Der MVP nutzt Full Sync. Der Code und das Datenmodell enthalten bereits `sync_state.delta_link`, damit eine spätere Version `/users/delta` verwenden kann. Delta Sync reduziert Datenverkehr und Rate-Limit-Risiko. Das Graph-Modul beachtet rudimentär `Retry-After` bei `429` und nutzt begrenzte Backoff-Retries bei `5xx`.

## iOS / MDM CardDAV Anforderungen

- iOS CardDAV benötigt für echte Geräte einen erreichbaren HTTPS-Endpunkt.
- Produktiv ist ein gültiges TLS-Zertifikat erforderlich.
- Selbstsignierte Zertifikate sind für produktive iOS/MDM-Verteilung nur eingeschränkt sinnvoll.
- Basic Auth Benutzername und App-Passwort werden im MDM-Profil hinterlegt.
- CardDAV Payload Type: `com.apple.carddav.account`.
- Server URL Beispiel: `https://carddav.example.com/carddav/demo/default/`.
- Port: `443`.
- SSL: aktivieren.

Beispielwerte für eine spätere MDM-Verteilung:

```text
AccountDescription: Company GAL
HostName: carddav.example.com
Port: 443
PrincipalURL: /carddav/demo/default/
Username: ios-demo
Password: generated-app-password
UseSSL: true
```

## Schnellstart (lokal)

In wenigen Minuten von Null zu einem abfragbaren CardDAV-Adressbuch – ganz ohne
Microsoft 365. Für die ersten Schritte genügen Testkontakte; der Microsoft Graph
Sync ist optional und in [Microsoft App Registration](#microsoft-app-registration-und-externe-anforderungen)
beschrieben.

### Voraussetzungen

- Rust Toolchain (`cargo`, Edition 2021)
- Docker und Docker Compose (für PostgreSQL)
- `curl` und optional `openssl` (zum Erzeugen der Secrets)

#### Rust installieren (rustup, empfohlen)

Rust am besten über `rustup` installieren, **nicht** über die Distro-Pakete:
`apt install cargo` liefert je nach Distribution ein zu altes `rustc`
(z. B. 1.85), das die Abhängigkeiten nicht baut.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version   # sollte aktuelles stable anzeigen
```

> Die mitgelieferte `Cargo.lock` ist zusätzlich auf rustc-1.85-kompatible
> Versionen gepinnt. Mit einem aktuellen `rustup`-stable funktioniert der Build
> aber ohne Sonderbehandlung.

#### Docker installieren

Installationsanleitung für die jeweilige Distribution:
<https://docs.docker.com/engine/install/>. Auf Debian/Ubuntu nutzt du das
`docker-compose-plugin` (Befehl `docker compose`, nicht `docker-compose`).

Wenn dein Benutzer nicht in der `docker`-Gruppe ist, brauchen die folgenden
`docker`-Befehle ein vorangestelltes `sudo`. Dauerhaft ohne `sudo`:

```bash
sudo usermod -aG docker "$USER"   # danach neu anmelden
```

### Schritt 1: PostgreSQL starten

```bash
docker compose up -d postgres   # ggf. mit sudo
```

### Schritt 2: Konfiguration anlegen

```bash
cp .env.example .env
```

Wichtig: `ADMIN_API_TOKEN` und `SUPPRESSION_SECRET` müssen jeweils mindestens
**32 Zeichen** lang sein, sonst startet der Dienst nicht. Die Platzhalter in
`.env.example` erfüllen diese Länge für lokale Tests. Für alles andere starke
Werte erzeugen:

```bash
echo "ADMIN_API_TOKEN=$(openssl rand -hex 32)" >> .env
echo "SUPPRESSION_SECRET=$(openssl rand -hex 32)" >> .env
```

> Hinweis: `SUPPRESSION_SECRET` muss stabil bleiben. Ein nachträglicher Wechsel
> erzeugt andere HMAC-Werte und entwertet bestehende Suppression-Einträge.

### Schritt 3: API starten

```bash
cargo run -p api
```

Die Datenbank-Migrationen werden beim Start automatisch ausgeführt. Sobald
`starting galcard api` im Log erscheint, läuft der Dienst auf der in `BIND_ADDR`
konfigurierten Adresse (Standard `127.0.0.1:3000`).

Der Prozess läuft im Vordergrund. Für die folgenden `curl`-Beispiele ein
zweites Terminal öffnen – oder den Dienst mit `cargo run -p api &` im
Hintergrund starten.

### Schritt 4: Health-Check

```bash
curl http://localhost:3000/health
```

### Schritt 5: Komplettbeispiel ohne Microsoft 365

Das folgende Skript legt einen Tenant, einen Testkontakt und einen CardDAV-User
an und ruft das Adressbuch anschließend per CardDAV ab. Es nutzt das Admin-Token
aus `.env.example`; bei eigenen Secrets entsprechend anpassen.

```bash
ADMIN_TOKEN="dev-admin-token-change-me-to-32plus-chars"
BASE="http://localhost:3000"

# 1) Tenant anlegen und dessen ID merken
TENANT_ID=$(curl -s -X POST "$BASE/admin/tenants" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"slug":"demo","display_name":"Demo Tenant"}' | jq -r .id)

# 2) Testkontakt anlegen
curl -s -X POST "$BASE/admin/tenants/$TENANT_ID/test-contacts" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"Max Mustermann","given_name":"Max","surname":"Mustermann","email":"max.mustermann@example.com","company_name":"Example GmbH","job_title":"IT Administrator"}'

# 3) CardDAV-User anlegen – das Passwort wird nur EINMAL zurückgegeben
curl -s -X POST "$BASE/admin/tenants/$TENANT_ID/carddav-users" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"ios-demo","addressbook_slug":"default"}'

# 4) Adressbuch per CardDAV abfragen (Passwort aus Schritt 3 einsetzen)
curl -s -X REPORT -u ios-demo:DAS_PASSWORT_AUS_SCHRITT_3 \
  "$BASE/carddav/demo/default/"
```

### Optional: kompletter Stack im Container

```bash
docker compose --profile api up --build
```

## Konfiguration

Alle Secrets kommen aus Umgebungsvariablen:

```text
DATABASE_URL
ADMIN_API_TOKEN
MICROSOFT_TENANT_ID
MICROSOFT_CLIENT_ID
MICROSOFT_CLIENT_SECRET
SUPPRESSION_SECRET
RUST_LOG
BIND_ADDR
PUBLIC_BASE_URL
```

Siehe `.env.example`. Dort stehen nur Entwicklungsplatzhalter, keine echten Secrets.

## Datenbank Migrationen

Migrationen liegen in `migrations/`:

- `0001_init.sql`: Tenants, Connections, Contacts, Sync State, Audit Log.
- `0002_carddav_users.sql`: lokale CardDAV App-User mit Argon2id-Hash.
- `0003_privacy_suppression.sql`: Suppression HMAC-Liste.

Die API führt Migrationen beim Start per `sqlx::migrate!` aus. Alternativ können sie mit `sqlx-cli` angewendet werden.

## API Beispiele

### Tenant anlegen

```bash
curl -X POST http://localhost:3000/admin/tenants \
  -H "Authorization: Bearer dev-admin-token-change-me-to-32plus-chars" \
  -H "Content-Type: application/json" \
  -d '{
    "slug": "demo",
    "display_name": "Demo Tenant",
    "microsoft_tenant_id": "00000000-0000-0000-0000-000000000000"
  }'
```

### Sync starten

```bash
curl -X POST http://localhost:3000/admin/tenants/{tenant_id}/sync \
  -H "Authorization: Bearer dev-admin-token-change-me-to-32plus-chars"
```

### Testkontakt anlegen

```bash
curl -X POST http://localhost:3000/admin/tenants/{tenant_id}/test-contacts \
  -H "Authorization: Bearer dev-admin-token-change-me-to-32plus-chars" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "Max Mustermann",
    "given_name": "Max",
    "surname": "Mustermann",
    "email": "max.mustermann@example.com",
    "business_phone": "+49 1234 56789",
    "company_name": "Example GmbH",
    "job_title": "IT Administrator"
  }'
```

### CardDAV User erstellen

```bash
curl -X POST http://localhost:3000/admin/tenants/{tenant_id}/carddav-users \
  -H "Authorization: Bearer dev-admin-token-change-me-to-32plus-chars" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "ios-demo",
    "addressbook_slug": "default"
  }'
```

Antwort enthält das Passwort nur einmal:

```json
{
  "id": "...",
  "username": "ios-demo",
  "password": "generated-password-show-once"
}
```

### Suppression anlegen

```bash
curl -X POST http://localhost:3000/admin/privacy/suppressions \
  -H "Authorization: Bearer dev-admin-token-change-me-to-32plus-chars" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "max.mustermann@example.com",
    "scope_type": "global",
    "reason": "manual_opt_out"
  }'
```

Die Klartext-E-Mail wird nicht gespeichert und nicht zurückgegeben.

Optionale Felder:

- `scope_type`: `global` (alle Tenants) oder `tenant`.
- `scope_value`: bei `scope_type=tenant` die betroffene Tenant-ID.
- `expires_at`: RFC-3339-Zeitstempel (z. B. `2026-12-31T23:59:59Z`); nach Ablauf
  greift die Suppression nicht mehr. Ohne Angabe gilt sie unbefristet.

## CardDAV Beispiele

### vCard abrufen

```bash
curl -u ios-demo:generated-password \
  http://localhost:3000/carddav/demo/default/{contact_id}.vcf
```

### PROPFIND

```bash
curl -X PROPFIND -u ios-demo:generated-password \
  -H "Depth: 1" \
  http://localhost:3000/carddav/demo/default/
```

### REPORT

```bash
curl -X REPORT -u ios-demo:generated-password \
  -H "Content-Type: application/xml" \
  http://localhost:3000/carddav/demo/default/
```

## Infrastruktur

MVP benötigt:

```text
Linux Server
Docker oder native Rust Runtime
PostgreSQL
öffentliche HTTPS URL für iOS Tests
Microsoft 365 Tenant mit Admin Consent
```

### Reverse Proxy

Empfohlen sind Caddy, nginx oder Traefik für:

- TLS-Terminierung.
- Forwarded Headers.
- kleine Request Body Limits.
- nur notwendige Ports öffnen.
- Rate Limiting pro Client-IP. Der eingebaute Limiter ist ein einfaches globales
  Sicherheitsnetz (Requests pro Sekunde über alle Clients) und schützt nicht
  gezielt vor Brute-Force auf einzelne CardDAV-Konten. Ein Per-IP-Limit am
  Reverse Proxy ergänzt dies.

## Datenschutzgrenzen

- galcard speichert GAL-Daten lokal in PostgreSQL.
- galcard ist read-only gegenüber iOS und schreibt nicht nach Microsoft 365 zurück.
- galcard liest keine E-Mails, Kalender, Teams Chats oder Dateien.
- Opt-Out/Suppression speichert keine Klartext-E-Mail, sondern HMAC-SHA256.
- HMAC ist Pseudonymisierung, keine vollständige Anonymisierung.
- Logs sollen keine personenbezogenen Kontaktinformationen enthalten.
- Fotos sind im MVP deaktiviert.
- Audit-Metadaten dürfen keine E-Mail-Adressen, Telefonnummern, Tokens oder vCards enthalten.

## Security Hinweise

- Admin API nutzt im MVP ein statisches Bearer Token aus `ADMIN_API_TOKEN`.
- `ADMIN_API_TOKEN` und `SUPPRESSION_SECRET` müssen jeweils mindestens 32 Zeichen lang sein.
- Token- und Passwortvergleiche erfolgen in konstanter Zeit (kein Timing-Leak).
- CardDAV nutzt Basic Auth mit lokalen App-Passwörtern.
- Passwörter werden nur als Argon2id Hash gespeichert.
- App-Passwörter werden nur bei Erstellung oder Rotation im Klartext angezeigt.
- `SUPPRESSION_SECRET` muss stabil und geheim bleiben; ein Wechsel erzeugt andere HMAC-Werte.
- Produktiv müssen Tokens und Secrets verschlüsselt gespeichert werden.
- Produktiv ist HTTPS zwingend erforderlich.

## Nicht im MVP enthalten

- vollständiges SaaS Admin Portal
- Billing
- LDAP/LDAPS OnPrem Connector
- Exchange OnPrem
- EWS
- Kontaktfotos
- Schreibzugriff per CardDAV
- Benutzer-Self-Service-Portal
- automatische MDM-Profilgenerierung
- vollständige CalDAV/WebDAV Suite
- vollständige Microsoft OAuth Admin Consent UI

## Roadmap

### 0.1 MVP

- Microsoft Graph Full Sync
- PostgreSQL Cache
- Read-only CardDAV
- Basic Auth CardDAV User
- Privacy Suppression HMAC

### 0.2

- Delta Sync
- Gruppenfilter
- Feldfilter pro Tenant
- MDM Profil Generator
- bessere iOS Kompatibilitätstests

### 0.3

- Admin Web UI
- Microsoft Admin Consent Flow
- Token Verschlüsselung produktiv
- Tenant Self-Service

### 0.4

- OnPrem LDAP Connector
- Exchange Hybrid Szenarien
- Audit Dashboard
- Lösch- und Exportfunktionen

### 1.0

- SaaS-fähig
- Produktiver Betrieb
- AVV/DSGVO Dokumentation
- Regionale Datenhaltung
