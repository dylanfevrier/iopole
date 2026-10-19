# iopole

Outil en ligne de commande pour la plateforme Iopole.

## Sommaire

- [Installation](#installation)
  1. [Installer les outils de compilation](#1-installer-les-outils-de-compilation)
  2. [Installer Rust](#2-installer-rust)
  3. [Récupérer les sources](#3-récupérer-les-sources)
  4. [Compiler et installer `iopole`](#4-compiler-et-installer-iopole)
  5. [Se connecter à Iopole](#5-se-connecter-à-iopole)
  6. [Synchroniser les factures](#6-synchroniser-les-factures)
  7. [Explorer les factures](#7-explorer-les-factures)
  8. [Autres usages](#8-autres-usages)
- [Utilisation](#utilisation)
  - [Authentification](#authentification)
  - [document sync](#document-sync)
  - [document search](#document-search)
  - [document stat](#document-stat)
  - [document status](#document-status)
  - [entity search](#entity-search)
  - [entity update-claim](#entity-update-claim)
  - [entity register](#entity-register)
  - [directory french](#directory-french)
- [Serveur MCP](#serveur-mcp)
- [Développement](#développement)

## Installation

Ces étapes mènent d'une machine sans Rust jusqu'aux factures synchronisées en
local, sur macOS ou Linux.

### 1. Installer les outils de compilation

SQLite et la bibliothèque TLS sont compilés depuis leurs sources C pendant la
construction : il faut donc un compilateur C.

- **macOS** : installer les outils en ligne de commande de Xcode :

  ```sh
  xcode-select --install
  ```

- **Debian / Ubuntu** :

  ```sh
  sudo apt update
  sudo apt install build-essential git curl
  ```

- **Fedora** :

  ```sh
  sudo dnf install gcc make git curl
  ```

### 2. Installer Rust

Installer Rust avec [rustup](https://rustup.rs), l'installateur officiel, en
gardant les options par défaut :

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Ouvrir un nouveau terminal (ou lancer `. "$HOME/.cargo/env"`) pour que `cargo`
soit dans le `PATH`, puis vérifier :

```sh
cargo --version
```

Rust 1.88 ou plus récent est requis. Le projet fixe le canal `stable` dans
`rust-toolchain.toml` : rustup l'installe, avec `rustfmt` et `clippy`, au
premier lancement de `cargo` dans le projet. Pour mettre à jour une
installation plus ancienne : `rustup update`.

### 3. Récupérer les sources

```sh
git clone git@github.com:dylanfevrier/iopole.git
cd iopole
```

### 4. Compiler et installer `iopole`

```sh
cargo install --path .
```

La première compilation télécharge les dépendances et compile SQLite : elle
prend quelques minutes. Le binaire est installé dans `~/.cargo/bin`, que rustup
a ajouté au `PATH`. Vérifier :

```sh
iopole --version
iopole --help
```

Pour mettre à jour plus tard : `git pull`, puis de nouveau
`cargo install --path .`.

### 5. Se connecter à Iopole

Récupérer auprès d'Iopole le **client**, le **secret** et le **customer ID** de
l'accès API, puis :

```sh
iopole auth login
```

Les trois valeurs sont demandées l'une après l'autre (le secret ne s'affiche
pas) et enregistrées dans `~/.config/iopole/secrets.json`, lisible par vous
seul. Voir [Authentification](#authentification).

### 6. Synchroniser les factures

Récupérer les factures depuis une date de début (format `AAAA-MM-JJ`) jusqu'à
maintenant :

```sh
iopole document sync --from 2026-09-01
```

Une barre de progression s'affiche, puis le nombre de factures récupérées :

```
████████████████████████████████████████ 100%
1234 invoices retrieved. 1000 inbound. 234 outbound.
```

La même commande peut être relancée à tout moment : elle ne récupère que ce
qui manque depuis la dernière fois. Voir [document sync](#document-sync) pour
`--to`, `--direction` et `--refresh`.

### 7. Explorer les factures

Les factures sont maintenant dans la base locale :

```sh
iopole document stat               # nombre de factures par état et statut
iopole document search 352862346   # factures d'une entreprise, par SIREN
```

### 8. Autres usages

Chaque commande a sa section détaillée plus bas, et `iopole <commande> --help`
liste toutes ses options.

**Factures**

```sh
# Filtrer la recherche : rôle, état, statut, période, nombre de résultats
iopole document search 352862346 --role buyer --state VALID,DELIVERED --from 2026-09-01 --limit 50

# Afficher le SIREN ou le BCE de chaque partie, à réutiliser avec `entity search`
iopole document search --identifiers

# Sortie JSON, par exemple pour `jq`
iopole document search 352862346 --json

# Historique des statuts d'une facture, demandé en direct à Iopole
iopole document status 01a10b05-e3fd-74c8-b9cd-ce8e4c012777

# Rechercher de nouveau une période et réécrire les factures modifiées
iopole document sync --from 2026-09-01 --refresh
```

Voir [document search](#document-search), [document status](#document-status)
et [document sync](#document-sync).

**Entités de l'opérateur**

```sh
# Entités enregistrées chez Iopole pour un identifiant (SIREN, SIRET, BCE...)
iopole entity search 352862346

# Remplacer la revendication d'une entité (rôle `admin` requis)
iopole entity update-claim 980217632 --company-uid 6071797

# Publier un identifiant sur l'annuaire français et sur Peppol
iopole entity register 982740128
```

Voir [entity search](#entity-search), [entity update-claim](#entity-update-claim)
et [entity register](#entity-register).

**Annuaire français**

```sh
# Rechercher une entreprise, cliente de l'opérateur ou non, par SIREN ou par nom
iopole directory french 352862346
iopole directory french --name "BVS RENO*"
```

Voir [directory french](#directory-french).

**Assistants IA**

```sh
# Donner à Claude Code l'accès en lecture seule aux factures locales
claude mcp add iopole -- iopole mcp
```

Voir [Serveur MCP](#serveur-mcp) pour Codex et Claude Desktop.

**Déconnexion**

```sh
# Supprimer les identifiants et la base locale
iopole auth logout
```

## Utilisation

Les commandes ci-dessous sont écrites avec `cargo run --`, pour les lancer
depuis les sources sans installer ; une fois `iopole` installé, remplacer
`cargo run --` par `iopole`.

```sh
cargo run -- --help
cargo run -- auth login
cargo run -- auth logout
cargo run -- document sync --from 2026-09-01 [--to 2026-10-01] [--direction inbound|outbound|both] [--refresh]
cargo run -- document search 352862346
cargo run -- document stat [--direction inbound|outbound|both]
cargo run -- document status 01a10b05-e3fd-74c8-b9cd-ce8e4c012777
cargo run -- entity search 352862346
cargo run -- entity update-claim 980217632 --company-uid 6071797 [--direction inbound|outbound|both]
cargo run -- entity register 982740128 [--network domestic-fr|peppol-international|both]
cargo run -- directory french 352862346
cargo run -- mcp
```

### Authentification

`auth login` demande le client, le secret et le customer ID Iopole, et les
enregistre en clair dans `~/.config/iopole/secrets.json`, lisible uniquement
par son propriétaire (fichier en `600`, dossier en `700`). Toutes les
commandes refusent de lire le fichier dès que d'autres utilisateurs peuvent le
lire.

`auth logout` supprime les identifiants et la base locale ; le fichier
disparaît avec eux.

### document sync

`document sync` enregistre les informations générales des factures Iopole
(jamais les moyens de paiement) dans une base SQLite locale,
`~/.local/share/iopole/iopole.db`, lisible uniquement par son propriétaire
(fichier en `600`, dossier en `700`). La base n'est pas chiffrée : elle
s'ouvre avec n'importe quel outil SQLite, par exemple
`sqlite3 ~/.local/share/iopole/iopole.db`. Pour protéger les données en cas de
vol de la machine, activer le chiffrement du disque (FileVault sur macOS,
LUKS sur Linux).

Les versions précédentes chiffraient la base : `iopole` refuse de l'ouvrir et
demande de la supprimer. Supprimer le fichier (ou lancer `iopole auth logout`
puis `iopole auth login`), puis relancer `iopole document sync`.

- Les factures sont recherchées par plages de leur identifiant UUID v7, un
  jour à la fois ; les jours qui dépassent les 1200 résultats que renvoie une
  recherche sont coupés en deux, jusqu'à une minute.
- Chaque recherche reprend aussi les derniers 10 % de la fenêtre précédente,
  pour qu'aucune facture ne tombe entre deux recherches. Les factures
  retrouvées ne voient que leur statut mis à jour, et seulement s'il n'est pas
  plus ancien que celui enregistré.
- Les plages déjà synchronisées ne sont jamais recherchées de nouveau : lancer
  `document sync` deux fois ne fait que combler les trous. La dernière heure
  reste ouverte et est recherchée de nouveau au lancement suivant.
- `--refresh` recherche de nouveau toute la période, plages déjà synchronisées
  comprises, et réécrit chaque facture enregistrée qui diffère de ce que
  renvoie Iopole maintenant : statut, données métier et JSON brut, même quand
  le statut paraît plus ancien. La commande indique alors aussi combien de
  factures ont été mises à jour.
- La production est utilisée par défaut. Définir `IOPOLE_AUTH_URL` et
  `IOPOLE_PDP_URL` (et éventuellement `IOPOLE_AUTH_ENDPOINT`) pour viser un
  autre environnement, par exemple `https://auth.ppd.iopole.fr` et
  `https://api.ppd.iopole.fr`.

`document sync` affiche sa progression, puis le nombre de factures
récupérées :

```
████████████████████████████████████████ 100%
1234 invoices retrieved. 1000 inbound. 234 outbound.
```

### document search

`document search` cherche les factures d'une entreprise, comme vendeur ou
acheteur, uniquement dans la base locale. Il compare la valeur de l'adresse
électronique quel que soit son schéma : `352862346` trouve `0225:352862346` et
`0002:352862346`, `0123456789` trouve `0208:0123456789`. Sans identifiant, la
recherche couvre toutes les entreprises, et `--role` est refusé.

```sh
cargo run -- document search [352862346 [--role seller|buyer]] [--direction inbound|outbound|both] \
    [--state VALID,DELIVERED] [--status REJECTED] [--from 2026-09-01] [--to 2026-10-01] [--limit 50] \
    [--identifiers]
```

`--identifiers` affiche le vendeur et l'acheteur sous la forme
`352862346 - CCLS`, pour pouvoir passer l'identifiant à `entity search`.

Quand rien ne correspond, la commande suggère la commande `document sync` à
lancer.

`--json` affiche les factures trouvées telles qu'Iopole les a renvoyées lors de
la synchronisation, indentées, sous forme de liste (`[]` quand rien ne
correspond).

### document stat

`document stat` compte les factures de la base locale par état, chacun détaillé
par dernier statut, du plus fréquent au moins fréquent. `--direction` garde un
seul flux de factures ; par défaut, les deux sont comptés :

```
STATE          STATUS       COUNT
DELIVERED      -             1000
-              RECEIVED       580
-              ISSUED         420
WRONG_ROUTING  -             3000
-              (no status)   3000

4000 invoices.
```

### document status

`document status` demande à Iopole, en direct, l'historique des statuts d'une
facture, à partir de son UUID tel qu'affiché par `document search`, du plus
récent au plus ancien :

```
ID:           01a10b05-e3fd-74c8-b9cd-ce8e4c012777
Seller:       Leroy Merlin France (SIREN 384560942)
Buyer:        BVS RENOVATION (SIREN 980217632)

DATE (UTC)           STATUS
2026-10-05 09:45:07  PAYMENT_RECEIVED (212)
```

- L'historique ne contient que les statuts du cycle de vie échangés sur le
  réseau. Les statuts de la plateforme, comme `MADE_AVAILABLE` ou `REJECTED`,
  n'y figurent pas : une facture peut les afficher dans `document search` et
  n'avoir aucun historique.
- Les statuts désignent les parties de façon générique (`VENDEUR`,
  `ACHETEUR`) : les noms viennent de la base locale quand la facture est
  synchronisée, les SIREN viennent des statuts.
- Iopole répond à un UUID inconnu par un historique vide, comme pour une
  facture sans statut.
- `--json` affiche l'historique tel qu'Iopole le renvoie, indenté, sans lire la
  base locale.

### entity search

`entity search` demande à Iopole, en direct, quelles entités de l'opérateur ont
la valeur d'identifiant donnée (SIREN, SIRET, BCE...). Il les affiche comme
`directory french`, avec le company UID, le pays, et pour chaque réseau la date
depuis laquelle chaque adresse est disponible. Il indique quand l'entreprise
n'est pas enregistrée.

`--json` affiche les entités trouvées telles qu'Iopole les renvoie, indentées,
sous forme de liste (`[]` quand rien ne correspond).

### entity update-claim

`entity update-claim` remplace, en direct, la façon dont l'opérateur revendique
une de ses entités. Il trouve l'entité par sa valeur d'identifiant (SIREN,
SIRET, BCE...) comme `entity search`, et refuse quand aucune ou plusieurs
correspondent :

- `--company-uid` (obligatoire) est renvoyé par Iopole dans l'en-tête
  `X-Company-UID` des webhooks concernant cette entité.
- `--direction` limite la revendication à un seul flux de factures ; par
  défaut, elle couvre les deux.

Iopole remplace toute la configuration : tout autre en-tête, paramètre de
requête ou de chemin défini sur la revendication est supprimé. Le rôle `admin`
est nécessaire.

### entity register

`entity register` publie, en direct, un identifiant d'une des entités de
l'opérateur sur les réseaux de facturation électronique, pour que les autres
plateformes lui acheminent les factures. Il trouve l'entité par sa valeur
d'identifiant comme `entity update-claim`, et enregistre l'identifiant sous le
schéma qu'Iopole lui connaît (`0002` pour un SIREN). `--network` choisit
`domestic-fr` (l'annuaire français) ou `peppol-international` ; par défaut, il
enregistre sur les deux, l'annuaire français d'abord.

Un réseau qui publie déjà l'identifiant est signalé par `already up to date`,
et le suivant est quand même enregistré. Lancer `entity search` ensuite pour
voir sur quels réseaux l'identifiant est disponible.

### directory french

`directory french` cherche, en direct, dans l'annuaire français de la
facturation électronique, n'importe quelle entreprise, cliente de l'opérateur
ou non. Il prend un SIREN par défaut, ou d'autres critères, combinés quand il
y en a plusieurs :

```sh
cargo run -- directory french 352862346
cargo run -- directory french --name OBAT [--siret 35286234600012] [--type legal-unit|office] \
    [--limit 20] [--offset 0] [--json]
```

`--name` ne tient pas compte de la casse, demande au moins 3 caractères et
accepte le joker `*` (`--name "BVS RENO*"`). Les détails de plateforme sont
demandés (`withPlatformDetails`). Chaque entrée affiche :

```
Name:         MUCHA CONCIERGERIE
Type:         Office (PRIMARY)
Legal unit:   MADAME FLORIA DJOMO-NDJILO (SIREN 944870435)
Address:      62 RUE DANIELLE CASANOVA, 93200 SAINT-DENIS

TYPE               IDENTIFIER           NETWORK
OFFICE_IDENTIFIER  0009:94487043500012  -
```

Les entrées sont séparées par une ligne de `=`. Un identifiant enregistré sur
plusieurs réseaux occupe une ligne par réseau. Une colonne vide sur toutes les
lignes, comme `AVAILABLE SINCE` quand l'annuaire omet cette date, n'est pas
affichée.

L'annuaire ne donne pas le nombre total de résultats : quand une page est
pleine, la commande indique quel `--offset` affiche la suivante.

`--json` affiche la réponse d'Iopole telle que reçue, indentée, au lieu des
entrées, par exemple pour déboguer ou pour la passer à `jq`.

## Serveur MCP

`iopole mcp` met les factures locales à disposition des assistants IA, sous
forme d'outils MCP sur stdio. Il n'appelle jamais Iopole : les factures
viennent de `document sync`.

| Outil | Rôle |
|---|---|
| `search_invoices` | liste les factures comme `document search`, en JSON |
| `invoice_stats` | compte les factures par état et dernier statut comme `document stat` |

Les deux sont en lecture seule. Une fois `iopole` installé (voir
[Installation](#installation)) :

```sh
# Claude Code
claude mcp add iopole -- iopole mcp
```

```toml
# Codex : ~/.codex/config.toml
[mcp_servers.iopole]
command = "iopole"
args = ["mcp"]
```

```json
// Claude Desktop : claude_desktop_config.json, avec le chemin absolu
{ "mcpServers": { "iopole": { "command": "/Users/<vous>/.cargo/bin/iopole", "args": ["mcp"] } } }
```

Pour essayer les outils à la main : `npx @modelcontextprotocol/inspector iopole mcp`.

## Développement

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
