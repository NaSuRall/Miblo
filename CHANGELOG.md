# Changelog

## 0.9.1 — 2026-11-18

- Interface en anglais par défaut, avec sélection English / Français par
  `Maj+L`, la palette d’actions ou `:language` / `:langue`.
- Choix de langue appliqué immédiatement et conservé entre les lancements dans
  les préférences utilisateur, sans modifier les projets ni arrêter l’API.
- Traduction des menus, formulaires, aides, états API/BDD et assistants CLI.
- Tests terminal dans les deux langues : création, modèles, routes, requêtes
  HTTP, changement pendant l’exécution de l’API et erreurs de préférences.
- README repensé pour crates.io, guide détaillé séparé et métadonnées de la
  crate actualisées : description, mots-clés CLI/TUI/API et aide de la commande.

## 0.9.0 — bêta publique, 2026-11-18

- Nom public de la crate : `miblo`, exécutable `miblo`.
- Archives Windows, Linux et macOS avec sommes SHA-256.
- Tests du CLI et de l’installation sur Linux, macOS et Windows ; vérification
  de Rust 1.88, version minimale prise en charge.
- Tests des API générées SQLite/MySQL, de l’interface terminal et de la
  compatibilité avec les anciens modèles dans GitHub Actions.
- Installation du paquet `.crate` dans un dossier isolé, puis génération avec
  les templates publiés, cache hors ligne et compilation de l’API produite.
- Paquet Cargo limité aux sources, tests Rust, documentation et licence.
- Mise à jour de la dépendance transitive retirée `chacha20` vers 0.10.2.
- Lecture des noms de tables compatible avec les métadonnées MySQL 8.4.
- Erreur de template incomplet indiquant le fichier manquant, sur tous les systèmes.

## 0.9.0 — 2026-11-17

- Routes liées aux modèles : lister, consulter, créer, modifier et supprimer
  avec le CRUD et les requêtes SQL existants.
- Formulaires de test HTTP préremplis, sans saisie de JSON ; mises à jour
  partielles et choix des relations depuis les lignes existantes.
- Explication des réponses 501 des routes personnalisées et accès à leur
  fonction métier dans l’éditeur.
- Reprise du formulaire après une erreur de template ; source GitHub proposée
  par défaut pour les nouveaux projets.
- Templates externes 0.6.0 pour SQLite et MySQL.
