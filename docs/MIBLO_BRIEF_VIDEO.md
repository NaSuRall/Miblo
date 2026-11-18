# Miblo — Présentation du projet et brief pour une vidéo publicitaire

> Document à transmettre à Claude pour concevoir une vidéo de présentation de Miblo : scénario, voix off, storyboard, textes à l’écran et animation.
>
> État de référence : 17 novembre 2026, CLI 0.9.0, templates Rust 0.6.0. Première bêta publique en préparation. Les fonctions décrites comme disponibles existent dans le projet ; cela ne signifie pas que cette version est déjà publiée sur crates.io.

## 1. Miblo en une phrase

**Miblo permet de créer et faire évoluer le socle d’une API Rust depuis une interface dans le terminal : modèles, base de données, relations, routes et tests HTTP, avec du code lisible que le développeur peut reprendre librement.**

Promesse publicitaire proposée :

**« Ton idée. Tes modèles. Ton API. »**

Signature complémentaire :

**« Le socle de ton API, directement dans ton terminal. »**

Le nom à afficher est **Miblo**. La commande est `miblo`. Le grand mot animé au démarrage apparaît sous la forme **MIBLO**, en bleu.

## 2. Le problème auquel Miblo répond

Commencer une API implique souvent de répéter les mêmes étapes : créer l’arborescence, connecter une base, écrire les modèles, préparer les migrations, déclarer les routes, relier les handlers aux requêtes SQL, puis préparer des requêtes pour vérifier le résultat.

Cette préparation prend de la place avant même de commencer la partie spécifique au projet.

Miblo regroupe ces opérations dans un espace de travail au clavier. Le développeur décrit ce dont il a besoin ; Miblo construit une base cohérente, puis lui laisse la main sur un projet Rust classique.

Le gain à mettre en scène est concret : **passer de « je dois préparer toute la structure » à « je peux commencer à développer mon application »**.

## 3. À qui s’adresse le projet ?

- Aux développeurs Rust qui veulent démarrer rapidement une API.
- Aux indépendants et aux petites équipes qui recréent régulièrement les mêmes fondations.
- Aux personnes qui construisent un prototype, un outil interne, un back-office ou un projet personnel.
- Aux développeurs qui aiment le terminal et les interfaces pilotées au clavier.
- Aux personnes qui veulent comprendre et modifier le code produit.

Ces profils décrivent le public visé. Ils ne constituent pas des statistiques d’utilisateurs ou des témoignages clients.

## 4. L’expérience utilisateur

### Un espace de travail dans le terminal

L’utilisateur lance :

```sh
miblo
```

Une courte animation écrit **MIBLO en grand et en bleu**, puis laisse place à une interface plein écran. L’animation dure moins d’une seconde et peut être passée.

L’interface s’inspire des habitudes de navigation d’outils comme Neovim : flèches, raccourcis, palette d’actions et commandes précédées de `:`. Il ne s’agit pas d’un éditeur de code complet intégré.

L’espace de travail réunit :

- Les modèles et ressources du projet.
- L’explorateur des fichiers.
- Les routes et leur outil de test HTTP.
- L’état de l’API et de la base de données.
- Le schéma de la base et les relations entre tables.
- Les logs de lancement et de compilation.
- Un petit aperçu du code, que l’on peut masquer.

Le code peut s’ouvrir dans l’éditeur habituel avec une touche. Miblo prépare et organise le projet ; l’utilisateur continue à travailler avec ses outils.

### Les raccourcis utiles pour une démonstration

| Touche | Action |
| --- | --- |
| Flèches ou `j` / `k` | Naviguer |
| `Tab` | Changer de panneau |
| `Espace` | Ouvrir la palette d’actions |
| `n` | Créer une API |
| `o` | Ouvrir un projet |
| `a` | Ajouter un modèle, sa table et son CRUD |
| `m` | Ajouter des champs ou des relations à un modèle existant |
| `g` | Créer une route |
| `r` / `s` | Démarrer / arrêter l’API |
| `x` ou `Entrée` sur une route | Tester la route |
| `b` | Configurer la base et appliquer les migrations |
| `d` | Consulter le schéma de la base |
| `p` dans le schéma | Basculer entre schéma réel et modèles déclarés |
| `e` | Ouvrir le code dans son éditeur |
| `v` | Afficher ou masquer l’aperçu du code |
| `?` | Afficher l’aide |

Des commandes classiques restent disponibles pour les scripts : `miblo new`, `miblo add`, `miblo edit`, `miblo db`, etc.

## 5. Ce que Miblo sait faire aujourd’hui

### 5.1 Générer une API Rust

La génération actuelle repose sur :

- **Rust** pour le langage.
- **Axum** pour les routes HTTP.
- **SQLx** pour l’accès à la base et les migrations.
- **SQLite ou MySQL** pour le stockage.
- **Handlebars** pour rendre les fichiers à partir de templates versionnés.

L’utilisateur choisit le nom du projet, son emplacement, son port et sa base. Depuis l’interface, il peut créer une API vide puis ajouter ses ressources progressivement.

Le résultat est un véritable projet Rust, avec son `Cargo.toml`, ses sources, ses migrations, sa configuration et un README pratique en français.

### 5.2 Choisir sa base de données

**SQLite** est le parcours le plus simple pour une démonstration : le fichier de base et la configuration sont préparés automatiquement. Aucun serveur de base de données supplémentaire n’est nécessaire. Les données persistent dans le fichier SQLite du projet.

**MySQL** dispose d’une configuration guidée : hôte, port, nom de base, utilisateur et mot de passe. Un serveur MySQL accessible doit déjà exister. Miblo peut initialiser la base et appliquer les migrations avec les permissions appropriées.

Le choix du moteur se fait à la création du projet. La conversion automatique d’une API existante de SQLite vers MySQL, ou l’inverse, ne fait pas partie du parcours actuel.

### 5.3 Définir les modèles et obtenir le CRUD

Exemple : créer un modèle `User` avec `name` et `email`.

Miblo génère ensemble :

1. La structure Rust du modèle.
2. Les types des requêtes et leur validation.
3. Une migration SQL pour créer la table.
4. Les fonctions d’accès aux données.
5. Les routes pour créer, lire, modifier et supprimer les enregistrements.

Le CRUD est le groupe d’opérations de base sur une ressource :

| Action | Exemple de route |
| --- | --- |
| Lister | `GET /users` |
| Créer | `POST /users` |
| Lire un élément | `GET /users/{id}` |
| Modifier partiellement | `PATCH /users/{id}` |
| Supprimer | `DELETE /users/{id}` |

Les types de champs disponibles sont texte court, texte long, entier, nombre décimal et booléen. Des options permettent de rendre un champ facultatif, unique ou privé dans les réponses HTTP. Chaque modèle reçoit automatiquement une clé primaire entière `id`.

Un champ privé est omis des réponses. Cette option ne chiffre pas les données et ne met pas en place une authentification.

### 5.4 Créer les relations entre modèles

Exemple : un utilisateur appartient à un groupe.

```text
Group
  id       ← clé primaire
  name

User
  id       ← clé primaire
  name
  email
  group_id → Group.id
```

Dans le formulaire, l’utilisateur choisit le modèle cible. Miblo prépare le champ de relation, son type, l’index, la contrainte SQL et les adaptations du CRUD.

Il est possible de définir une relation facultative, une relation unique et le comportement lors de la suppression de la ligne référencée : bloquer la suppression, supprimer les lignes liées ou mettre la référence à `NULL`, selon les options choisies.

Les liens sont réellement vérifiés par la base de données. Les routes utilisent les identifiants des relations ; elles ne produisent pas automatiquement des jointures complexes ou des objets imbriqués.

### 5.5 Faire évoluer un modèle existant

L’utilisateur peut ajouter un champ ou une relation depuis l’interface, même après avoir commencé à utiliser son API.

Les nouveaux champs ajoutés par ce parcours sont facultatifs. Les anciennes lignes restent présentes et reçoivent `NULL` pour le nouveau champ. Une nouvelle migration SQL est générée ; les anciennes migrations ne sont pas réécrites.

Exemple : ajouter `nickname` à `User`, appliquer la migration, puis remplir ce champ progressivement avec une requête PATCH.

Miblo prend en compte les modifications manuelles du code grâce à une fusion avec la version initialement générée. Si une modification entre en conflit, il arrête la génération et signale le problème. Des sauvegardes des fichiers remplacés sont conservées.

Formulation publicitaire adaptée : **« Fais évoluer tes modèles en conservant tes données existantes. »**

Cette formulation concerne les ajouts pris en charge. Ne pas promettre que toute modification de schéma est automatiquement sans risque. Renommer ou supprimer des colonnes, changer leurs types et modifier les règles d’une relation existante demandent encore une migration écrite séparément. Les sauvegardes de code ne sont pas des sauvegardes de base de données.

### 5.6 Générer une route liée à un modèle

Le créateur de route propose de choisir un modèle et une action : lister, lire un élément, créer, modifier ou supprimer.

Exemple : choisir `User`, l’action **Créer**, puis le chemin **`/members`**.

Miblo prépare une route **`POST /members`** qui utilise le modèle User, sa validation, ses fonctions métier et ses requêtes SQL. Elle fonctionne comme une autre entrée vers la création d’un utilisateur.

Le développeur n’a pas à écrire une requête SQL pour cette opération CRUD standard.

Un second parcours permet de créer une route avec une **logique métier personnalisée** : les fichiers du handler, de la requête et de la fonction métier sont préparés, puis le développeur complète le comportement dans son éditeur. Ce squelette renvoie initialement un statut « non implémenté ».

Pour la vidéo, utiliser le parcours **modèle + action CRUD** afin de montrer une réponse immédiatement fonctionnelle. Un nom comme `/users/active` n’invente pas automatiquement un filtre « utilisateurs actifs » : cette règle doit être programmée.

### 5.7 Tester les routes sans écrire de JSON

C’est un des moments les plus visuels de la démonstration.

En sélectionnant une route POST liée à un modèle, l’utilisateur obtient un formulaire préconstruit avec les champs attendus et des valeurs d’exemple. Il peut les ajuster puis sélectionner **« Envoyer la requête »**.

Miblo construit le JSON, conserve les bons types et affiche le code HTTP, le temps de réponse et le contenu retourné.

- Les champs facultatifs peuvent rester à `NULL`.
- Les références peuvent proposer des enregistrements existants du modèle lié.
- Une requête PATCH permet de choisir seulement les champs à modifier.
- Une suppression demande une confirmation.
- L’envoi reste une action explicite de l’utilisateur.

La préconstruction s’appuie sur les modèles et les routes connus de Miblo. Elle ne déduit pas automatiquement tous les contrats de requêtes Rust écrits manuellement. Les payloads personnalisés imbriqués et les en-têtes d’authentification personnalisés ne sont pas encore pris en charge par le testeur.

Message à l’écran : **« Choisis une route. Vérifie les champs. Envoie. »**

### 5.8 Voir l’état de l’API et de la base

L’interface donne un retour visuel pendant le travail :

| Couleur | Signification générale |
| --- | --- |
| Vert | API joignable ; base accessible et migrations à jour sur sa carte respective |
| Orange | Démarrage, redémarrage nécessaire, initialisation ou migrations à appliquer |
| Rouge | Erreur de processus, de réponse HTTP, de connexion ou de cohérence des migrations |
| Gris | API arrêtée, projet non ouvert ou vérification en attente |

Les états API et BDD sont distincts. Le vert confirme un contrôle de fonctionnement ; il ne certifie pas la sécurité ou la correction de toutes les règles métier.

L’utilisateur peut démarrer et arrêter l’API, suivre ses logs, consulter le schéma réel et comparer celui-ci aux modèles déclarés.

La vue de schéma montre les tables, colonnes, types, clés primaires, clés étrangères et règles de suppression. Il ne faut pas la présenter comme un éditeur complet de toutes les lignes de la base.

### 5.9 Garder du code compréhensible

La lisibilité du résultat fait partie de la promesse de Miblo.

```text
mon-api/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── miblo.yaml
├── migrations/
└── src/
    ├── main.rs
    ├── lib.rs
    ├── config.rs
    ├── database.rs
    └── resources/
        ├── mod.rs
        └── user/
            ├── mod.rs
            ├── handlers.rs
            ├── model.rs
            ├── requests.rs
            ├── service.rs
            └── repository.rs
```

Chaque fichier a un rôle identifiable :

| Fichier | Rôle |
| --- | --- |
| `mod.rs` | Déclarer et assembler les routes du modèle |
| `handlers.rs` | Recevoir les appels HTTP et produire les réponses |
| `model.rs` | Décrire les données |
| `requests.rs` | Décrire et valider les données reçues |
| `service.rs` | Accueillir les règles métier |
| `repository.rs` | Contenir les requêtes SQL |

Le chemin d’une requête est simple à expliquer : **HTTP → handler → service → base de données**.

Les fichiers sont formatés avec `rustfmt`, accompagnés de commentaires ciblés et d’un README en français. Le projet généré peut être compilé et exécuté sans Miblo. Il peut être versionné, adapté et déployé avec les outils habituels du développeur.

Message publicitaire : **« Un socle prêt à reprendre. Du code que tu comprends. »**

## 6. Comment le projet est organisé techniquement

Miblo sépare le programme de génération de ses templates.

| Élément | Responsabilité |
| --- | --- |
| Dépôt `Miblo` | Application Rust, interface terminal, commandes, génération, gestion des projets, accès au schéma et tests HTTP |
| Dépôt `miblo_templates` | Packs Handlebars versionnés pour produire les API SQLite et MySQL |
| GitHub Releases des templates | Distribution des archives et de leurs fichiers de contrôle SHA-256 |
| Projet généré | Code de l’utilisateur, configuration, migrations et métadonnées nécessaires à son évolution |

Les templates **ne sont pas intégrés au binaire**. Ils sont téléchargés à la demande, vérifiés, puis mis en cache. Une version précise est associée au projet. Les futures générations peuvent réutiliser ce cache hors ligne.

Une URL de distribution configurable et des dossiers locaux sont disponibles pour d’autres usages ou pour développer les templates. Le parcours normal n’exige pas de récupérer manuellement le dépôt des templates.

Le mode hors ligne suppose que le template requis a déjà été téléchargé. La compilation Rust demande également que les dépendances Cargo soient disponibles.

Miblo n’héberge pas l’API produite. L’API s’exécute là où le développeur décide de la lancer.

## 7. La puissance à montrer, avec des preuves visuelles

| Bénéfice | Preuve à l’écran | Formulation possible |
| --- | --- | --- |
| Moins de préparation répétitive | Un modèle devient fichiers, table et routes | « Décris tes données. Miblo prépare le socle. » |
| Un parcours cohérent | Navigation entre modèles, routes et schéma | « Ton API, au même endroit. » |
| Une base facile à démarrer | Choix SQLite puis création terminée | « Démarre simplement avec SQLite. » |
| Des relations concrètes | `User.group_id → Group.id` dans le schéma | « Relie tes modèles. » |
| Des routes exploitables | Route liée à User, requête puis réponse 201 | « Du modèle à la requête. » |
| Des tests plus accessibles | Formulaire POST prérempli | « Teste sans écrire le JSON. » |
| Une API qui évolue | Ajout d’un champ, nouvelle migration, anciennes lignes conservées | « Fais évoluer ton projet. » |
| Un code facile à reprendre | Ouverture de `service.rs` dans l’éditeur | « Continue dans ton éditeur. » |
| Autonomie du projet | Arborescence Rust et `cargo run` | « Le projet reste entre tes mains. » |

La « puissance » à communiquer vient de l’enchaînement de ces opérations. Aucun benchmark, nombre d’utilisateurs ou pourcentage de temps gagné n’est établi par ce document.

## 8. Périmètre de la promesse et état de diffusion

### À présenter comme disponible dans le projet

L’interface terminal, la génération Rust/Axum/SQLx, SQLite et MySQL, les modèles, le CRUD, les relations, les ajouts de champs, les migrations, le schéma, les routes liées aux modèles, les formulaires HTTP et l’ouverture du code dans un éditeur.

### À présenter comme en préparation

La première bêta publique et la qualification de l’installation sur plusieurs systèmes. Des tests unitaires, d’intégration et des scénarios sur API réelles sont en place ; les vérifications automatisées de distribution sont en cours de consolidation.

### À ne pas annoncer comme livré

- La génération Node.js ou d’autres langages : ambition d’extension, sans engagement de date dans ce brief.
- Une authentification, des permissions ou une sécurisation complète de l’API générée.
- Un hébergement ou un déploiement automatique en production.
- Un éditeur de code complet à l’intérieur de Miblo.
- Des règles métier complexes inventées à partir du nom d’une route.
- Une génération par intelligence artificielle : le moteur actuel utilise des templates.
- Une application métier complète sans intervention du développeur.

**Le but du produit est de fournir rapidement la structure de base.** La logique spécifique, la sécurité, l’authentification et la mise en production de chaque API sont prises en charge par son développeur.

La phrase « en quelques secondes » peut qualifier la génération des fichiers lorsque les prérequis et les templates sont disponibles. La première installation et la première compilation des dépendances Rust peuvent prendre plusieurs minutes. Ne pas montrer un chronomètre qui prétendrait mesurer l’installation complète.

## 9. Démonstration de référence à filmer

### Projet de démonstration

Nom : **`team-api`**. Base : **SQLite**. Cas d’usage : des utilisateurs rattachés à des groupes.

Modèles :

```text
Group
  name: string

User
  name: string
  email: string, unique
  group_id: entier facultatif, référence vers Group.id
```

Route supplémentaire : **`POST /members`**, associée à **User → Créer**.

Évolution du modèle : ajouter **`nickname`**, texte facultatif.

### Ordre réel des actions

1. Lancer `miblo` et filmer l’animation bleue.
2. Créer `team-api` avec SQLite et les templates GitHub Releases.
3. Ajouter `Group`, puis `User` et sa relation facultative vers Group.
4. Ouvrir le schéma avec `d` et montrer la clé étrangère.
5. Créer la route `POST /members` avec l’action **Créer** du modèle User.
6. Démarrer l’API avec `r` et attendre son état vert.
7. Tester `POST /groups` et créer un groupe « Studio ».
8. Tester `POST /members` : ajuster les valeurs du formulaire, choisir le groupe créé, envoyer et montrer la réponse **201** avec un identifiant.
9. Appeler `GET /users` pour montrer que la donnée est réellement enregistrée. Ne pas supposer qu’un `GET /members` a été créé avec la route POST.
10. Arrêter l’API avec `s`, ajouter `nickname` à User avec `m`, puis montrer l’état orange signalant la migration en attente.
11. Appliquer la migration via `b`, redémarrer et vérifier que l’utilisateur créé est toujours présent, avec `nickname` à `null`.
12. Ouvrir le code dans l’éditeur avec `e`, montrer les fichiers séparés et finir sur le nom Miblo.

Pour une vidéo courte, les étapes peuvent être accélérées ou présentées sous forme de coupes. Préparer les dépendances avant le tournage et ne pas faire passer une compilation coupée au montage pour une mesure de vitesse réelle.

Utiliser uniquement des données de démonstration : « Léa », `lea@example.test`, « Studio ». Les captures ne doivent pas afficher de secrets, de mots de passe réels ou de chemins personnels inutiles.

## 10. Direction artistique proposée

**Ambiance :** un outil de développeur clair, rapide et maîtrisé. Le terminal est au centre de l’image.

- Fond sombre, proche de l’interface réelle.
- Bleu principal : **`#4696FF`**, proche du bleu du mot animé au démarrage.
- Blanc doux pour la lisibilité ; vert et orange réservés aux états de l’application.
- Police monospace lisible pour le terminal, police simple pour les accroches.
- Peu de texte à l’écran : une idée par plan.
- Zooms courts sur une sélection, une relation ou une réponse HTTP.
- Transitions simples, synchronisées avec les validations au clavier.
- Sons de touches et de confirmation discrets ; musique instrumentale énergique sans gêner la voix.

Conserver une interface crédible. Utiliser des captures réelles pour les parcours produit. Les animations explicatives peuvent schématiser une table ou une relation, sans faire croire qu’un écran absent existe dans l’application.

Ne pas ajouter de faux assistants IA, graphiques de performances, boutons de déploiement, tableaux de clients ou logos de partenaires.

## 11. Proposition de vidéo principale — 60 à 75 secondes

Objectif : faire comprendre le produit, montrer une opération qui fonctionne et donner envie de découvrir le projet.

| Temps | Image / action | Voix off proposée | Texte à l’écran |
| --- | --- | --- | --- |
| 0–6 s | Quelques fichiers de départ, puis un terminal prêt | « Une nouvelle API. Encore les modèles, les routes, la base… et toute la structure à préparer. » | « Une idée d’API ? » |
| 6–11 s | `miblo`, animation MIBLO bleue, ouverture de l’espace de travail | « Avec Miblo, commence directement dans ton terminal. » | « Miblo » |
| 11–20 s | Choix SQLite ; création de Group puis User en montage rapide | « Choisis ta base, décris tes modèles. Miblo génère les tables, les migrations et les routes CRUD. » | « Modèles → Tables → Routes » |
| 20–28 s | Sélection de Group comme relation ; zoom sur le schéma | « Relie tes données et retrouve tes clés étrangères dans le schéma. » | « Des modèles reliés » |
| 28–38 s | Création de `POST /members`, modèle User, action Créer ; API verte | « Ajoute une route pour ton modèle. Son accès aux données est déjà préparé. » | « User → POST /members » |
| 38–48 s | Formulaire prérempli, envoi, réponse 201 ; lecture de l’utilisateur | « Pour la tester, les champs sont prêts. Ajuste les valeurs, envoie et vois la réponse. » | « Teste sans écrire le JSON » |
| 48–58 s | Ajout de nickname ; migration orange puis appliquée ; donnée conservée | « Ton projet évolue ? Ajoute un champ et génère la migration en gardant tes données existantes. » | « Fais évoluer ton API » |
| 58–67 s | Ouverture du dossier user et de service.rs dans l’éditeur | « Tu récupères du code Rust organisé, lisible, prêt à accueillir ta logique métier. » | « Du code que tu comprends » |
| 67–75 s | Retour au mot MIBLO ; lien GitHub | « Miblo. Ton idée. Tes modèles. Ton API. Découvre le projet sur GitHub. » | « github.com/NaSuRall/Miblo » |

Le rythme de voix et la durée des plans peuvent être ajustés. Les informations de ce document font foi ; les phrases publicitaires sont des propositions créatives.

## 12. Variante courte — environ 15 secondes

**Images :** animation bleue → modèle User → relation → formulaire POST → 201 → code organisé → logo.

**Voix off :**

> « Ton idée d’API commence ici. Avec Miblo, crée tes modèles, relie tes données et génère tes routes depuis ton terminal. Teste sans écrire le JSON. Reprends du code Rust lisible. Miblo : ton idée, tes modèles, ton API. »

**Textes à l’écran :** « Crée » → « Relie » → « Teste » → « Développe » → « Découvre Miblo sur GitHub ».

## 13. Formats et éléments à produire

Pour la vidéo principale, préparer un format **16:9, 1920 × 1080**. Prévoir une adaptation **9:16, 1080 × 1920** avec des cadrages plus serrés sur les formulaires et les réponses.

Livrables attendus de Claude :

1. Un script final avec une voix off naturelle en français.
2. Un storyboard plan par plan, avec durées et transitions.
3. Les textes à l’écran et les sous-titres.
4. Une liste des captures réelles nécessaires.
5. Les éléments animés à créer : mot MIBLO, flèches modèle/table/route, schéma simplifié, cartouche HTTP 201.
6. Une version principale et une version courte.
7. Si son environnement sait fabriquer une vidéo, les sources du montage ou de l’animation et les exports ; sinon un dossier de production directement utilisable.

Ne pas supposer l’existence de captures, d’un logo vectoriel ou d’une vidéo finale. Les demander ou utiliser une représentation explicitement illustrative si ces ressources ne sont pas fournies.

## 14. Appel à l’action et liens

Appel à l’action adapté à l’état actuel : **« Découvre Miblo sur GitHub et suis la préparation de la bêta. »**

- Projet : <https://github.com/NaSuRall/Miblo>
- Templates : <https://github.com/NaSuRall/miblo_templates>
- Templates versionnés : <https://github.com/NaSuRall/miblo_templates/releases/tag/v0.6.0>

Pour une démonstration depuis les sources, avec Rust, Cargo, les outils natifs et `rustfmt` installés :

```sh
cargo install --git https://github.com/NaSuRall/Miblo.git --locked
miblo
```

La version à filmer doit être préparée et vérifiée avant la capture. La publication de la crate est une étape distincte : ne pas afficher une installation crates.io comme disponible avant de l’avoir vérifiée. Ne pas annoncer de date de sortie, de prix ou de disponibilité sur un système non confirmé.

## 15. Prompt prêt à transmettre à Claude

```text
Tu dois concevoir une vidéo publicitaire pour Miblo en utilisant le document
joint comme référence produit.

Miblo est un générateur de socles d’API Rust, piloté depuis une interface plein
écran dans le terminal. Il génère modèles, tables, migrations, relations et
routes CRUD avec Axum et SQLx, pour SQLite ou MySQL. Il permet de créer une route
liée à un modèle, de tester les requêtes avec des formulaires préremplis, de voir
les états API/BDD et de reprendre un code organisé dans son éditeur.

Public : développeurs, indépendants, petites équipes et créateurs de projets.
Ton : direct, clair, technique mais accessible. Français naturel, tutoiement.
Direction visuelle : terminal sombre, bleu Miblo #4696FF, textes courts,
navigation au clavier, captures produit lisibles, mouvements sobres.
Promesse principale : « Ton idée. Tes modèles. Ton API. »

Produis :
- une vidéo principale de 60 à 75 secondes, ou son dossier de production si
  ton environnement ne permet pas de produire le fichier vidéo ;
- un storyboard horodaté, une voix off et les textes à l’écran ;
- la liste précise des captures et ressources nécessaires ;
- une adaptation courte d’environ 15 secondes ;
- des indications pour les formats 16:9 et 9:16.

Construis la narration autour d’une démonstration : créer team-api en SQLite,
ajouter Group et User, relier User à Group, générer POST /members pour créer un
User, envoyer un formulaire prérempli, montrer une réponse 201, ajouter nickname
avec une migration, puis ouvrir le code Rust organisé.

Appuie chaque promesse sur une action visible. La vidéo doit donner envie et
montrer pourquoi le parcours fait gagner des étapes de préparation.

Respecte le périmètre du document : la bêta publique est en préparation ; ne
prétends pas que la crate de cette version est déjà publiée. N’invente aucune
fonction, aucun témoignage, chiffre d’adoption, benchmark ou partenariat.
Ne présente pas Node.js, l’authentification, une sécurité complète, un déploiement
automatique ou une génération par IA comme des fonctionnalités livrées.
La génération rapide concerne la structure ; la première compilation Rust peut
prendre plusieurs minutes. Le développeur ajoute sa logique métier et prépare
la sécurité et la mise en production de son API.

Utilise des captures réelles pour montrer le fonctionnement. Si elles manquent,
liste-les et distingue clairement les animations explicatives de l’interface
réelle. Termine par « Découvre Miblo sur GitHub » et le lien
https://github.com/NaSuRall/Miblo.
```

## 16. Sources de vérité dans le projet

Pour actualiser ce brief avant la diffusion :

- `README.md` : fonctionnalités, commandes, limites et installation.
- `src/tui/` : interface, formulaires, statuts, testeur HTTP et animation.
- `src/generation/` et `src/generation.rs` : création, ajout de routes et évolution des modèles.
- `src/database/` et `src/database.rs` : schéma, migrations et état de la base.
- `src/templates.rs` : téléchargement, cache, versions et intégrité des templates.
- `tests/` et `scripts/smoke_*.py` : vérifications du CLI, des API et de l’interface.
- Le dépôt `miblo_templates` : structure exacte du code produit.

Ce document donne une base factuelle et une proposition éditoriale. Les slogans, le montage et la direction artistique peuvent évoluer en gardant les capacités réelles du produit comme référence.
