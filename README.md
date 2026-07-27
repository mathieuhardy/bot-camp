# Serveur de test pour crawlers — Rust / Axum

Outil auto-hébergé (les utilisateurs le lancent/déploient eux-mêmes) permettant de tester le comportement d'un crawler/scraper face à : codes HTTP, headers, robots.txt, redirections, canonical, contenu HTML avancé, et **rate limiting / anti-bot**.


---

## 1. Principe d'architecture (à lire avant la liste de features)

`crawler-test.com` fonctionne avec des centaines de **pages statiques** codées en dur, une par cas de test. Pour un outil que des devs vont s'auto-héberger et vouloir étendre, c'est le mauvais modèle : il vaut mieux un **moteur générique piloté par des paramètres d'URL et un fichier de config**, plutôt que 200 handlers Rust copiés-collés.

Deux familles de routes à prévoir dès le départ :

- **Routes paramétriques** (la majorité) : le comportement est dans l'URL ou les query params.
  `GET /status/{code}`, `GET /redirect/{code}?to=/status/200&steps=3`, `GET /delay/{ms}`, `GET /headers?x-foo=bar`
- **Scénarios déclaratifs** (cas complexes ou "canon" qu'on veut nommer et documenter) : définis dans un fichier YAML/TOML chargé au démarrage (et rechargeable à chaud), qui génère la route, la réponse ET l'entrée correspondante dans `/robots.txt` si besoin.

Ça évite de recompiler à chaque nouveau cas de test et ça rapproche l'outil d'un vrai "test lab" plutôt que d'un musée de pages figées.

---

## 2. Liste exhaustive des fonctionnalités

### 2.1 Codes de statut HTTP & headers

- `GET /status/{code}` — retourne n'importe quel code de 100 à 999 (choix volontairement permissif, aligné sur ce que permet le crate `http` : ça couvre les non-standards type 419, 420, 440, 444, 449, 450, 494-499, 520, 598, 599, mais aussi les classes 6xx-9xx qui n'existent dans aucune RFC, utiles pour tester la robustesse d'un crawler face à un code totalement inattendu)
- Codes informationnels 1xx (100 Continue, 102 Processing) — nécessite un support bas niveau (hyper permet ça, à vérifier selon la version)
- `GET /headers/echo` — renvoie tous les headers reçus (façon `crawler_request_headers` et `crawler_user_agent` de crawler-test.com)
- `GET /headers/set?name=value&name2=value2` — force des headers de réponse arbitraires (Content-Type malformé, X-Robots-Tag, Cache-Control, HSTS, Content-Encoding annoncé mais absent, etc.)
- Simulation de **Content-Type malformé / incohérent** (header vs meta tag HTML)
- Simulation d'**encodage de caractères incohérent** (UTF-8 déclaré, Latin-1 servi, etc.)
- `GET /compression/{gzip|br|deflate|identity}` — tester la négociation de compression
- `GET /conditional` — support ETag / If-None-Match / Last-Modified / If-Modified-Since (304)
- `GET /auth/basic` — Basic Auth (401 puis 200 avec les bons identifiants)
- `GET /auth/bearer` — Bearer token simulé
- IP du client vu par le serveur (utile pour valider le forwarding derrière un proxy)
- `GET /large-response/{kb}` — taille de page contrôlée (test des limites de crawl)
- `GET /timeout/{ms}` — délai de réponse contrôlé (page load time)
- HTTP/1.0 vs HTTP/1.1 vs HTTP/2 — comportement de connexion, keep-alive
- Réponses **chunked** vs Content-Length fixe

### 2.2 Robots protocol

- `/robots.txt` généré dynamiquement à partir de la config (pas un fichier statique figé)
  - `Disallow`, `Allow`, `Crawl-delay`, `Sitemap:`, règles par `User-agent` (y compris UA custom type `Googlebot`, `DeepCrawlBot`, etc.)
  - Cas piégeux : ligne vide au milieu, `Allow` plus long/court/égal au `Disallow` correspondant, directives dupliquées, casse différente
- Meta robots dans le `<head>` : `noindex`, `nofollow`, `noarchive`, `nosnippet`, `none`, `noodp`, `noydir`, combinaisons multiples, casse variable
- `X-Robots-Tag` en header HTTP (équivalent meta robots mais côté header)
- **Conflits volontaires** : meta robots vs X-Robots-Tag contradictoires, robots.txt disallow + meta index contradictoires
- Blocage par `User-Agent` spécifique (tester qu'un crawler respecte bien son propre UA dans le disallow)
- Page bloquée par robots.txt mais quand même indexable via un lien externe (test de la gestion de ce cas)
- `Non-200 + noindex`, `Canonicalisé + noindex`, `Canonicalisé + code non-200` (combinaisons robots × canonical × status)

### 2.3 Redirects & canonical

- Redirections HTTP : 300, 301, 302, 303, 304, 305, 306 (réservé), 307, 308 — chacune avec sémantique différente à valider (méthode préservée ou non)
- Chaînes de redirection configurables : `GET /redirect-chain/{n}` avec n sauts
- Boucle infinie de redirection (`/redirect-loop`)
- Boucle en 2 étapes (A→B→A)
- Redirection vers une URL externe
- Redirection vers une URL interdite par robots.txt
- Redirection vers un 404
- Redirection relative vs absolue
- **Meta refresh** (`<meta http-equiv="refresh">`) avec délai variable, cible interne/externe/invalide
- **Header Refresh** (`Refresh:` en header HTTP plutôt qu'en meta tag)
- Redirection via JavaScript (`window.location`, fonction, `onclick`, `onchange`, concaténation d'URL, `pushState`)
- Balise canonical : self-référentielle, vers une autre page, relative/absolue, dans le `<head>` vs hors `<head>`, dupliquée, en conflit avec `og:url`, vers une URL externe, vers une URL bloquée par robots.txt
- Règles de normalisation d'URL à tester via canonical : casse du host (insensible), casse du path (sensible), ordre des query params (canonicalisé), valeur/clé de param (sensible à la casse), port par défaut (`:80`/`:443` = sans port), trailing slash, trailing dot, fragment (`#`) ignoré

### 2.4 Rate limiting / anti-bot (le cœur différenciant vs crawler-test.com)

C'est la partie qui n'existe pas vraiment sur crawler-test.com et qui a le plus de valeur ajoutée pour toi.

- `GET /ratelimit/{n}-per-{window}` — limite générique configurable (ex: 10 req / 60s) avec réponse 429 + `Retry-After`
- Algorithmes à proposer (au moins un, idéalement plusieurs en option) : fixed window, sliding window, **token bucket**, leaky bucket
- Rate limit par clé : par IP, par `User-Agent`, par clé d'API/header custom, global
- Ban temporaire après dépassement de seuil (ex: 3 fois 429 → ban 5 min avec 403)
- Ban permanent simulé (liste noire IP/UA en config)
- Allow-list / block-list explicite d'IP et d'User-Agent
- Détection de **crawl trop rapide** (moins de X ms entre deux requêtes de la même source)
- Détection d'**absence de crawl-delay respecté** (le serveur connaît le `Crawl-delay` qu'il a annoncé et log/bloque si non respecté)
- Simulation de **challenge anti-bot** : page qui renvoie un défi JS simple (façon "checking your browser"), avec cookie de validation ensuite
- Simulation de **honeypot** : lien caché (invisible visuellement, présent dans le HTML) que seul un bot naïf suivrait → détection + ban
- Détection par **absence de header attendu** (ex: pas d'`Accept-Language`, UA générique type `python-requests`) → 403
- Détection par **pattern de requêtes** (trop de 404 en peu de temps = scan) → ban temporaire
- Simulation de **CAPTCHA gate** (retourne toujours la même page de challenge tant qu'un cookie/paramètre "solved" n'est pas présent — pas un vrai captcha, juste la mécanique HTTP)
- Endpoint d'introspection `/ratelimit/status` — voir en direct son propre état (compteur courant, quota restant, TTL du ban) pour faciliter le debug pendant qu'on développe son crawler
- Reset manuel du compteur pour une clé donnée (endpoint admin)
- Rate limit variable selon le path (ex: `/api/*` plus strict que `/pages/*`)

### 2.5 Contenu & rendu HTML

- Titres : vide, manquant, dupliqué, trop long, avec espaces en trop, encodé
- Meta description : manquante, dupliquée, trop longue, avec `nosnippet`, via `http-equiv`
- H1 : absent, multiple, dans une image, en SVG
- Word count contrôlé, avec nombres/tirets/symboles/scripts inclus dans le comptage
- Encodage : URL avec caractères non-ASCII (multi-langues), double encodage, encodage incohérent entre déclaration et contenu réel
- HTML cassé : tag non fermé dans le `<head>`, balise non-head dans le head, `<link>` dans le `<body>`
- Contenu dupliqué entre deux pages (test de détection de duplicate content)
- Rendu JavaScript : injection de contenu en différé (texte, title, canonical, meta ajoutés en JS après chargement), AJAX qui remplit la page, `alert()`/`dialog` bloquant le rendu, script analytics/pub, timeout de rendu configurable
- Pagination : pages liées, pages non liées (orphelines), pagination + noindex
- Liens : cassés (internes/externes), nofollow, `rel` avec valeurs multiples/variations de guillemets, liens relatifs avec/sans `<base>`, liens en JS uniquement, liens vers fichiers non-HTML, liens vers URL malformées
- Hreflang : présent en HTML, présent en header HTTP (cohérent ou non)
- Mobile/AMP : page desktop séparée avec variations (H1, title, wordcount, liens différents), AMP self-canonical, AMP sans référence retour, configuration "responsive", "dynamic serving"
- Open Graph / Twitter Card : présents, incomplets, en conflit avec canonical

### 2.6 Fonctionnalités transverses

- Panneau de config (fichier YAML/TOML) rechargeable à chaud, qui pilote robots.txt + scénarios + règles de rate limit en un seul endroit
- Dashboard web minimal (liste des scénarios dispo, état des rate limits en cours) — optionnel, bonus
- Logs structurés de chaque requête (IP, UA, path, code retourné, règle appliquée) pour rejouer/analyser le comportement du crawler testé
- Export des logs en JSON/CSV pour analyse post-mortem
- Mode "replay" : rejouer une séquence de requêtes enregistrée
- On veut pouvoir laisser le user ajouter des endpoints basés sur ceux existant. Par exemple un `get /route?k=v` pourrait devenir un alias `get /route/bad_headers`.

---

## 3. Roadmap — du plus facile au plus difficile

### Phase 0 — Squelette du projet
- [x] Setup Axum + Tokio, routing de base, `tower-http` (tracing, compression, CORS)
- [x] Structure de projet en modules (voir section 4)
- [x] Un seul test end-to-end qui passe (`GET /health` → 200). Pour les tests, on évitera de lancer un serveur, mais on profitera de la possibilité de tester directement une App Axum.
- [x] Ajouter un fichier de CI pour github pour tester : formating via cargo fmt, le build en release, les tests via cargo nextest.
- [x] Ajouter un Dockerfile organisé en layers
- [x] Commencer un tutoriel dans le dossier `docs/tutorial` qui permettra de prendre en main toutes les fonctionnalités une par une.
    => on veut quelque chose de standard : get started (install, setup, deployment), tutorial (minimal), API (toutes les routes, features doccumentées à fond).

### Phase 1 — Codes HTTP & headers (le plus simple, forte valeur immédiate)
- [x] `/status/{code}` générique
- [x] `/headers/echo` et `/headers/set`
- [x] `/delay/{ms}`
- [x] `/large-response/{kb}`
- [x] Basic Auth simple

**Difficulté : faible.** Tout est stateless, pas de logique métier complexe. C'est le meilleur point d'entrée pour valider l'archi de routing générique.

### Phase 2 — Redirects & canonical
- [x] Redirections 301/302/307/308 paramétrables, chaînes, boucles
- [x] Header Refresh (`/redirect/refresh`)
- [x] Meta refresh (`/redirect/meta-refresh`) — moteur de templates : MiniJinja
- [x] Canonical tag (`/canonical` : self-référentiel/cross-page, relative/absolue, dupliqué, hors `<head>`, conflit `og:url`)
- [x] Cas de normalisation d'URL (`/normalize?url=...` : redirige en 301 vers la forme normalisée — casse du scheme/host, port par défaut, dot-segments, trailing slash, dots du host, ordre des query params, fragment — chaque règle optionnelle togglable via query param)

**Difficulté : faible à moyenne.** Nécessite de générer du HTML dynamique proprement — bon moment pour choisir et mettre en place le moteur de templates que tu garderas pour tout le reste.

### Phase 3 — Robots.txt & meta robots
- [x] `/robots.txt` servi depuis un état en mémoire (`PUT /robots.txt` pour le configurer, texte brut, puisqu'un crawler le fetch toujours sans query string) plutôt que depuis un fichier de config — voir la note ci-dessous
- [x] Meta robots + X-Robots-Tag, avec les cas de conflit (`/robots/meta?directives=...&x_robots_tag=...`)
- [ ] Lien entre scénarios et règles robots.txt (un scénario "bloqué" doit automatiquement apparaître dans le disallow généré) — reporté : dépend du moteur de scénarios déclaratifs (section 1), qui n'existe pas encore

**Difficulté : moyenne.** La partie non triviale, c'est de garder `/robots.txt` et les pages **cohérents entre eux** sans dupliquer la config à deux endroits — d'où l'intérêt du moteur de scénarios déclaratifs introduit en section 1. Pour cette phase, on a choisi un état mutable en mémoire plutôt qu'un fichier YAML/TOML chargé au démarrage : plus simple, pas de nouvelle dépendance de parsing, et suffisant tant que le moteur de scénarios n'est pas là. Le lien automatique scénario ↔ robots.txt attendra ce moteur.

### Phase 4 — Rate limiting simple, en mémoire
- Middleware Tower qui limite par IP/UA avec un stockage in-memory (`dashmap` ou `moka`)
- 429 + `Retry-After`, ban temporaire simple
- Endpoint `/ratelimit/status` pour introspection

**Difficulté : moyenne.** Le piège classique ici est l'extraction fiable de l'IP client (proxy, `X-Forwarded-For`, `Forwarded`) — à traiter proprement dès le début plutôt que de le patcher plus tard.

### Phase 5 — Contenu HTML avancé & JS
- H1/titres/word count/duplicate content
- Rendu JS différé (nécessite de servir du JS qui modifie le DOM après coup — reste simple côté serveur, c'est juste du HTML+JS statique généré)
- Encodage multi-langues, HTML cassé volontairement

**Difficulté : moyenne.** Rien de conceptuellement dur, mais beaucoup de cas particuliers à couvrir proprement (c'est là que la liste de scénarios déclaratifs grossit vraiment).

### Phase 6 — Anti-bot avancé & rate limiting distribué
- Honeypots, détection de pattern de crawl trop rapide, challenge JS simulé
- Abstraction du store de rate limit derrière un trait (`RateLimitStore`), avec deux implémentations : in-memory (par défaut) et Redis (pour tenir la charge / multi-instance)
- Ban basé sur des règles composées (UA + fréquence + pattern de 404)

**Difficulté : élevée.** C'est ici que l'architecture doit être la plus soignée : bien découpler la logique de décision (les règles) du store (où sont stockés les compteurs), pour ne pas se retrouver à tout réécrire en passant de mémoire à Redis.

### Phase 7 — Observabilité, admin, packaging
- Logs structurés + export JSON/CSV, éventuellement métriques Prometheus
- Reload de config à chaud (watch du fichier YAML)
- Dashboard web minimal
- Packaging pour l'auto-hébergement : binaire statique (musl), image Docker, docker-compose avec profil Redis optionnel


### Phase 8 — Bonus

- [] Generic API allowing to configure status code, response headers, etc.
- [] Discovery d'urls (dans html).
- [] Setters for robots.txt, etc.

**Difficulté : élevée**, mais surtout par le volume de travail (pas par la complexité algorithmique) — c'est la phase "produit fini, prêt à distribuer".

---

## 4. Conseils techniques & architecturaux

### Structure de projet suggérée

```
crawler-test-server/
├── src/
│   ├── main.rs
│   ├── config.rs         # chargement + reload YAML/TOML (serde + notify)
│   ├── routes/
│   │   ├── status.rs
│   │   ├── headers.rs
│   │   ├── redirects.rs
│   │   ├── robots.rs
│   │   ├── content.rs
│   │   └── ratelimit.rs
│   ├── scenarios/
│   │   ├── registry.rs   # chargement des scénarios déclaratifs
│   │   └── model.rs      # structs serde représentant un scénario
│   ├── ratelimit/
│   │   ├── store.rs      # trait RateLimitStore
│   │   ├── memory.rs     # impl DashMap/Moka
│   │   └── redis.rs      # impl deadpool-redis (feature-gated)
│   └── templating.rs
├── config/
│   ├── default.yaml
│   └── scenarios/*.yaml
└── tests/
    └── integration.rs
```

### Crates à privilégier

| Besoin | Crate |
|---|---|
| Web framework | `axum` + `tokio` |
| Middleware transverse (compression, trace, CORS, timeout) | `tower`, `tower-http` |
| Config | `serde`, `serde_yaml` ou `toml`, `config` |
| Rate limiting générique (fixed/sliding window) | `governor` (solide, bien maintenu) |
| Store in-memory rapide avec TTL | `moka` (cache async avec expiration, plus adapté que `dashmap` brut pour du ban temporaire) |
| Store distribué | `deadpool-redis` ou `fred` |
| Templating HTML | `askama` (compilé, type-safe) ou `minijinja` (plus dynamique, utile si tu veux éditer les templates sans recompiler) |
| CLI / config par flags | `clap` |
| Logs structurés | `tracing` + `tracing-subscriber` (+ `tracing-appender` pour fichiers) |
| Métriques | `metrics` + `metrics-exporter-prometheus` |
| Reload config à chaud | `notify` (watch fichier) |
| Tests d'intégration HTTP | `tower::ServiceExt::oneshot` pour les tests in-process, `reqwest` pour des tests boîte noire réels |
| Snapshot testing (utile pour valider le HTML généré par les scénarios) | `insta` |

### Points d'architecture à ne pas négliger

**1. Extraction de l'IP client.**
Ne fais pas confiance à `SocketAddr` brut par défaut — beaucoup d'utilisateurs vont mettre ton outil derrière un reverse proxy (nginx, Caddy) pour tester du HTTPS ou des vrais headers. Prévois une option de config `trust_proxy_headers: bool` qui, si activée, lit `X-Forwarded-For`/`Forwarded` en confiance. Documente bien que c'est dangereux si le serveur est exposé directement sur Internet sans proxy devant (spoofing facile sinon).

**2. Découpler règles et stockage pour le rate limiting.**
Définis un trait `RateLimitStore` avec des méthodes genre `increment(key) -> Count`, `is_banned(key) -> bool`, `ban(key, duration)`. Implémente d'abord la version mémoire, ajoute Redis plus tard en feature flag Cargo (`--features redis`). Ça te permet de livrer une version "simple, zéro dépendance externe" par défaut, et une version "prod-like" en option — exactement ce que tu voulais dans ta réponse.

**3. Le moteur de scénarios doit être la source de vérité pour robots.txt.**
Si un scénario dit `blocked_by_robots: true`, le générateur de `/robots.txt` doit lire cette même liste de scénarios pour construire ses règles `Disallow`. Sinon tu vas te retrouver avec des incohérences entre ce que dit robots.txt et ce que fait réellement la page — exactement le genre de bug que crawler-test.com est censé traquer chez les autres, ce serait dommage de l'avoir toi-même.

**4. Cohérence des templates HTML.**
Beaucoup de scénarios (titres, canonical, meta robots, OG tags) ne sont que des variations d'un même squelette HTML. Un seul template paramétrable avec des "trous" (title, meta, body, canonical, script JS additionnel) évite de dupliquer 100 fichiers `.html` quasi identiques.

**5. Distribution en self-hosted.**
Comme il n'y aura pas de déploiement centralisé, priorité à :
- un **binaire statique** (target `x86_64-unknown-linux-musl` via `cross` ou `cargo-zigbuild`) pour que ce soit "download & run" sans dépendances système,
- une **image Docker** avec un `docker-compose.yml` proposant un profil optionnel Redis (`docker compose --profile redis up`),
- un fichier de config par défaut embarqué dans le binaire (via `include_str!`) pour que ça marche "out of the box" sans fichier externe obligatoire, avec possibilité de override par variable d'env ou fichier local.

**6. Tests.**
Pour un outil dont le métier est justement "produire des réponses HTTP exactes", les tests d'intégration sont le meilleur investissement : un test par scénario qui vérifie code, headers et corps exacts. `tower::ServiceExt::oneshot` te permet de tester sans vraiment ouvrir de socket, donc rapide en CI.

---

## 5. Résumé — par où commencer concrètement

1. Squelette Axum + un `/status/{code}` qui marche → tu valides toute la chaîne technique.
2. Redirects + canonical → tu introduces le templating.
3. Robots.txt dynamique lié aux scénarios → tu valides le modèle "source de vérité unique".
4. Rate limiting mémoire simple avec `governor` + `moka` → tu as la feature différenciante en version basique.
5. Contenu HTML avancé → tu remplis la couverture façon crawler-test.com.
6. Rate limiting avancé (Redis, honeypots, patterns) → tu vas au-delà de crawler-test.com.
7. Packaging et observabilité → tu rends l'outil distribuable proprement.
