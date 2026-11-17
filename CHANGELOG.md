# Changelog

## Unreleased — préparation de la bêta

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

Le dépôt et une Release de templates ne constituent pas une publication de la
crate. La bêta crates.io sera annoncée après validation de son paquet exact.
