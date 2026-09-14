# Contributing to Microknot

First off, thank you for considering contributing to Microknot! It is a project that thrives on community input, whether it's via bug reports, feature requests, documentation updates, or pull requests.

To keep the codebase clean and the workflow efficient, please follow these guidelines.

## Table of Contents

- Code of Conduct
- How Can I Contribute?
    - Reporting Bugs
    - Suggesting Enhancements
    - Submitting Pull Requests
- Development Workflow
    - Environment Setup
    - Coding Standards
- Contributing Node Types

## Code of Conduct

By participating in this project, you agree to maintain a professional, respectful, and inclusive environment. We are committed to making participation in Microknot a harassment-free experience for everyone.

## How Can I Contribute?

### Reporting Bugs

If you find a bug, please open an Issue on GitHub. For the most helpful report, include:

A clear title and description.
Steps to reproduce: A minimal workflow JSON that triggers the bug.
Expected vs. Actual behavior.
System Info: Your OS and the version of Microknot used.

### Suggesting Enhancements

We love new ideas! If you want to add a new node type (e.g., db.postgres or notify.telegram) or a new feature, please open an Issue first to discuss the design. This ensures your effort aligns with the project's architecture.

### Submitting Pull Requests

Fork the repository and create your branch from main.
Create a feature branch: Use descriptive names like feat/add-postgres-node or fix/retry-logic.
Update Documentation: If you change a feature, update the README.md or relevant docs.

## Coding Standards

To maintain high code quality, we follow these rules:

Formatting: Always run cargo fmt before committing.
Linting: Always run cargo clippy to ensure idiomatic Rust code.
Documetation: Use /// doc comments for all new public functions and structs.
AI Usage: While permited for code generation PR's with AI written text might get rejected. Always check the AI written code yourself code can be considered "Slop" will be rejected.

## Contributing Knot Types

The heart of Microknot is its extensible Knot system. When contributing a new knot type (e.g., transform.* or notify.*):

Define the Schema: Ensure the new node's parameters are clearly defined in the JSON schema.
Implement the Trait: Implement the core execution trait for your node.

Error Handling: Ensure your node returns descriptive errors rather than panicking.
Test the IO: If your node interacts with the network or filesystem, include integration tests that use mocked services where possible.

## License

By contributing to Microknot, you agree that your contributions will be licensed under the project's AGPLv3 license.

**Thank you for helping build the future of local-first automation!**