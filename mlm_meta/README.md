mlm_meta
========

Small crate defining the Provider trait and helper types for external
metadata providers (Goodreads, Hardcover, ...).

Purpose
- Provide a stable trait so server can query multiple providers and map
  results into existing `TorrentMeta`.

How to add a provider
- Implement `mlm_meta::Provider` and return `TorrentMeta` from `fetch`.
- Register the provider in server's `MetadataService` and map fields into
  `TorrentMeta` before persisting.
