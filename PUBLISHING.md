# Publishing danube-connect-core to crates.io

## Overview

Yes, you should publish `danube-connect-core` to crates.io. This makes it easy for connector developers to use your SDK without needing to clone the repository.

## Prerequisites

1. **Cargo account:** Create an account at https://crates.io
2. **API token:** Generate an API token from your account settings
3. **Authenticate locally:**
   ```bash
   cargo login
   # Paste your API token when prompted
   ```

## Pre-Publication Checklist

Before publishing, ensure:

- [ ] Version is set correctly in `Cargo.toml` (currently `0.1.0`)
- [ ] `README.md` exists in the crate directory
- [ ] `LICENSE` file exists in the repository
- [ ] `Cargo.toml` has proper metadata:
  - [ ] `description` - Brief description of the crate
  - [ ] `keywords` - Up to 5 keywords
  - [ ] `categories` - Relevant categories
  - [ ] `repository` - GitHub URL
  - [ ] `license` - License identifier (Apache-2.0)

### Current Status

✅ All metadata is properly configured in `danube-connect-core/Cargo.toml`:
```toml
description = "Core SDK for building Danube connectors"
keywords = ["danube", "messaging", "connector", "integration"]
categories = ["network-programming", "asynchronous"]
repository = "https://github.com/danube-messaging/danube-connect"
license = "Apache-2.0"
```

## Publishing Steps

### 1. Verify the Package

```bash
cd /Users/drusei/proj_pers/danube-connect/danube-connect-core

# Check what will be published
cargo package --list

# Verify it builds correctly
cargo build --release
```

### 2. Run Tests

```bash
cargo test
cargo test --doc
```

### 3. Publish

```bash
# Dry run (doesn't actually publish)
cargo publish --dry-run

# Actual publish
cargo publish
```

### 4. Verify on crates.io

After publishing, verify at:
```
https://crates.io/crates/danube-connect-core
```

## Publishing danube-connect-common

You may also want to publish `danube-connect-common` separately:

```bash
cd /Users/drusei/proj_pers/danube-connect/danube-connect-common
cargo publish
```

## Version Management

### Semantic Versioning

Follow semver for version bumps:
- **0.1.0** → Initial release (current)
- **0.1.1** → Bug fixes (patch)
- **0.2.0** → New features (minor)
- **1.0.0** → Stable release

### Updating Versions

Update the workspace version in root `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.0"  # Bump this
```

This automatically updates all member crates.

## Dependency Updates

After publishing, if you update `danube-client` or `danube-core` versions, you'll need to:

1. Update the version in root `Cargo.toml`
2. Publish a new version of `danube-connect-core`

Example:
```toml
# When danube-client 0.6.0 is released
danube-client = "0.6.0"
danube-core = "0.6.0"
```

Then:
```bash
cargo publish
```

## Using Published Crates

Once published, users can add to their `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.1"
danube-connect-common = "0.1"
```

Instead of cloning the repository.

## Documentation

The published crate will automatically generate docs at:
```
https://docs.rs/danube-connect-core/
```

Ensure your code has good doc comments (which it does!).

## Yanking Versions

If you need to remove a published version (security issue, etc.):

```bash
cargo yank --vers 0.1.0
```

This prevents new projects from using that version but doesn't break existing ones.

## CI/CD Integration

Consider adding to your GitHub Actions:

```yaml
name: Publish to crates.io

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
```

## Recommended Publishing Order

1. **First:** Publish `danube-connect-common` (no dependencies on other crates in the workspace)
2. **Second:** Publish `danube-connect-core` (depends on danube-connect-common)

This ensures dependency resolution works correctly.

## Summary

✅ **Ready to publish!** All metadata is configured correctly. Just:

```bash
# Test first
cargo test

# Publish common utilities
cd danube-connect-common
cargo publish

# Publish core SDK
cd ../danube-connect-core
cargo publish
```

Your crates will then be available on crates.io for all Danube connector developers to use!
