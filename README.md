# CardSync - Zentralisierte Kontaktverwaltung via CardDAV

**CardSync** soll eine Python-basierte Server-Software werden, die es ermöglicht, Kontaktinformationen aus verschiedenen Quellen zu aggregieren und diese über eine einzige CardDAV-URL bereitzustellen. Die Anwendung soll kompatibel sein mit Debian-basierten Linux-Systemen und kann optional in Docker-Containern betrieben werden.

## Funktionen

- **Integration mehrerer Datenquellen:**
  - Microsoft Active Directory
  - LDAP-Verzeichnisse
  - Azure Active Directory
  - Exchange (Online) Global Address List (GAL)
  - Exchange (Online) Öffentliche Ordner
  - Microsoft SQL Server
  - MySQL-Datenbanken
  - Microsoft CA oder LDAP-Zertifikatsdienst

- **Adressbuchverwaltung:**
  - Erstellung von Adressbüchern, die Kontakte aus mehreren Quellen bündeln
  - Zuweisung von Adressbüchern zu einzelnen Benutzern oder Benutzergruppen
  - Flexible Konfiguration über eine webbasierte Administrationsoberfläche

- **Benutzerauthentifizierung:**
  - LDAP-basierte Anmeldung (Basic Auth / LdapBind)
  - Benutzer erhalten nur Zugriff auf die ihnen zugewiesenen Kontakte

- **S/MIME-Unterstützung:**
  - Einbettung von S/MIME-Public Keys in die Kontakte
  - Zertifikatsabruf von Microsoft CA und LDAP-basierten Zertifikatsdiensten
  - Ermöglicht verschlüsselte E-Mail-Kommunikation auch außerhalb des Firmennetzwerks

- **Sichere Kommunikation:**
  - CardDAV-Dienst erreichbar über HTTPS auf Port 443
  - Administrationsoberfläche auf separatem Port für erhöhte Sicherheit

- **Plattformkompatibilität:**
  - Entwickelt für Debian-basierte Linux-Distributionen
  - Unterstützung für Docker-Container zur einfachen Bereitstellung

## Architekturübersicht

![Architekturdiagramm](docs/images/architektur.png)

## Voraussetzungen

- **Betriebssystem:** Debian oder kompatible Linux-Distribution
- **Python-Version:** Python 3.8 oder höher
- **Abhängigkeiten:** Siehe `requirements.txt`
