# ADR 0003: public-source licensing and release identity

Context: the user requested preparation for public GitHub distribution and chose GPL-3.0-or-later.

Decision: project code is GPL-3.0-or-later. Android public release signing uses a new private key stored outside the repository; the development key is never the public update identity. Version 0.2.0 distinguishes this release from the local 0.1.0 prototype.

Consequences: prototype users uninstall once. Future release updates must use the same preserved private signing key. Distribution includes corresponding source and third-party notices. Publishing a GitHub repository or an AUR entry is a separate action; no destination or account has been assumed.
