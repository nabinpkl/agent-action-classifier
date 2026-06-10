//! ADR-0017 Cedar speed spike: does embedded `cedar-policy` evaluation meet the
//! stage-1 budget (p99 < 100us) at a representative org-first policy shape?
//!
//! This is the adoption GATE, not the production engine: it builds the inheritance
//! chain (agent in user in team in org), an OWASP-annotated policy mix, and benches
//! ONLY `Authorizer::is_authorized` (entities/policies/request built once, outside the
//! timed loop, so we measure evaluation and not setup). It sweeps policy count and
//! entity count to characterize the curve, since Cedar eval scales with both.
//!
//! Reference-or-frontier (ADR-0006): ref Microsoft <0.1ms inline; Cedar's published
//! ~single-digit-us embedded eval. Run: `just bench` or `cargo bench -p policy_decision`.

use std::collections::{HashMap, HashSet};
use std::hint::black_box;

use cedar_policy::{
    Authorizer, Context, Entities, Entity, EntityUid, PolicySet, Request, RestrictedExpression,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn uid(s: &str) -> EntityUid {
    s.parse().expect("valid entity uid")
}

fn scope(id: &str, sensitivity: &str) -> Entity {
    let mut attrs = HashMap::new();
    attrs.insert(
        "sensitivity".to_string(),
        RestrictedExpression::new_string(sensitivity.to_string()),
    );
    Entity::new(uid(&format!(r#"DataScope::"{id}""#)), attrs, HashSet::new())
        .expect("valid scope entity")
}

/// The org graph: org <- team <- user <- N agents (parent edges = inheritance), plus
/// data scopes with a `sensitivity` attribute (the request's resource + padding).
fn make_entities(agent_count: usize, scope_count: usize) -> Entities {
    let org = uid(r#"Org::"acme""#);
    let team = uid(r#"Team::"eng""#);
    let user = uid(r#"User::"nabin""#);

    let mut es: Vec<Entity> = vec![
        Entity::new(org.clone(), HashMap::new(), HashSet::new()).unwrap(),
        Entity::new(team.clone(), HashMap::new(), HashSet::from([org])).unwrap(),
        Entity::new(user.clone(), HashMap::new(), HashSet::from([team])).unwrap(),
    ];
    for i in 0..agent_count {
        es.push(
            Entity::new(
                uid(&format!(r#"Agent::"a{i}""#)),
                HashMap::new(),
                HashSet::from([user.clone()]),
            )
            .unwrap(),
        );
    }
    // Named scopes the request/policies reference, plus padding to scale the store.
    es.push(scope("customers", "low"));
    es.push(scope("secrets", "high"));
    for i in 0..scope_count {
        es.push(scope(&format!("s{i}"), if i == 0 { "high" } else { "low" }));
    }
    Entities::from_entities(es, None).expect("valid entity store")
}

/// An OWASP-annotated policy mix: a hard `forbid` (deny-overrides), an attribute-
/// conditioned `permit`, then padding permits to grow the set to `count`.
fn make_policies(count: usize) -> PolicySet {
    let mut src = String::new();
    src.push_str(
        "@id(\"forbid-secrets\") @owasp(\"ASI05\")\n\
         forbid(principal, action == Action::\"write\", resource == DataScope::\"secrets\");\n",
    );
    src.push_str(
        "@id(\"permit-low\") @owasp(\"ASI05\")\n\
         permit(principal, action, resource) when { resource.sensitivity == \"low\" };\n",
    );
    for i in 0..count.saturating_sub(2) {
        src.push_str(&format!(
            "@id(\"p{i}\")\n\
             permit(principal == Agent::\"a{i}\", action == Action::\"read\", resource == DataScope::\"s{i}\");\n",
        ));
    }
    src.parse().expect("valid policy set")
}

/// Agent a1 writes the (low-sensitivity) customers scope: matches `permit-low`, so the
/// path evaluates an attribute condition and scans the set (an allow, not a fast deny).
fn make_request() -> Request {
    Request::new(
        uid(r#"Agent::"a1""#),
        uid(r#"Action::"write""#),
        uid(r#"DataScope::"customers""#),
        Context::empty(),
        None,
    )
    .expect("valid request")
}

fn bench_decide(c: &mut Criterion) {
    let authorizer = Authorizer::new();
    let request = make_request();

    let mut g = c.benchmark_group("cedar_is_authorized");

    // Sweep 1: policy count (entities fixed at a small org).
    for &policy_count in &[5usize, 25, 100] {
        let policies = make_policies(policy_count);
        let entities = make_entities(8, 32);
        g.bench_with_input(
            BenchmarkId::new("policies", policy_count),
            &policy_count,
            |b, _| {
                b.iter(|| black_box(authorizer.is_authorized(&request, &policies, &entities)));
            },
        );
    }

    // Sweep 2: entity count (policies fixed at 25).
    for &n in &[16usize, 150] {
        let policies = make_policies(25);
        let entities = make_entities(n, n);
        g.bench_with_input(BenchmarkId::new("entities", n), &n, |b, _| {
            b.iter(|| black_box(authorizer.is_authorized(&request, &policies, &entities)));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_decide);
criterion_main!(benches);
