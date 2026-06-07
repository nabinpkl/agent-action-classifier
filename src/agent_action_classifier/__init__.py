"""Python host for the agent-action governance classifier.

Holds the impure edges around the pure Rust PDP core: the enforcement adapter (PEP),
the context/approval source (PIP), the LLM judge, and the audit sink. Packaged by
concept as those land; empty for now (the host's first real module arrives with the
PyO3 binding to the core). See SPEC.md.
"""
