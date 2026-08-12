use std::collections::HashMap;

use serde_json::Value;

use crate::error::Result;
use crate::item::Item;
use crate::knots;
use crate::schema::KnotSchema;

pub struct Ctx {
    pub workflow_id: String,
}

pub trait Knot {
    fn run(&self, ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>>;
}

pub struct KnotDescriptor {
    pub kind: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub output_count: usize,
    pub input_count: usize,
}

pub type KnotFactory = fn(&Value) -> Box<dyn Knot>;
pub type SchemaFactory = fn() -> KnotSchema;

pub struct Registry {
    factories: HashMap<&'static str, (KnotDescriptor, KnotFactory, SchemaFactory)>,
}

impl Registry {
    pub fn default() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        registry.register(
            knots::log::DESCRIPTOR,
            knots::log::factory,
            knots::log::schema,
        );
        registry.register(
            knots::set::DESCRIPTOR,
            knots::set::factory,
            knots::set::schema,
        );
        registry.register(
            knots::delay::DESCRIPTOR,
            knots::delay::factory,
            knots::delay::schema,
        );
        registry.register(
            knots::filter::DESCRIPTOR,
            knots::filter::factory,
            knots::filter::schema,
        );
        registry.register(
            knots::http::DESCRIPTOR,
            knots::http::factory,
            knots::http::schema,
        );
        registry.register(
            knots::resend::DESCRIPTOR,
            knots::resend::factory,
            knots::resend::schema,
        );
        registry.register(
            knots::trigger::WEBHOOK,
            knots::trigger::webhook_factory,
            knots::trigger::webhook_schema,
        );
        registry.register(
            knots::trigger::SCHEDULE,
            knots::trigger::schedule_factory,
            knots::trigger::schedule_schema,
        );
        registry
    }

    pub fn register(
        &mut self,
        descriptor: KnotDescriptor,
        factory: KnotFactory,
        schema: SchemaFactory,
    ) {
        self.factories
            .insert(descriptor.kind, (descriptor, factory, schema));
    }

    pub fn descriptor(&self, kind: &str) -> Option<&KnotDescriptor> {
        self.factories.get(kind).map(|(d, _, _)| d)
    }

    pub fn create(&self, kind: &str, params: &Value) -> Option<Box<dyn Knot>> {
        self.factories.get(kind).map(|(_, f, _)| f(params))
    }

    pub fn schema(&self, kind: &str) -> Option<KnotSchema> {
        self.factories.get(kind).map(|(_, _, s)| s())
    }

    pub fn kinds(&self) -> Vec<&KnotDescriptor> {
        let mut kinds: Vec<&KnotDescriptor> = self.factories.values().map(|(d, _, _)| d).collect();
        kinds.sort_by_key(|d| d.kind);
        kinds
    }

    /// Every knot's descriptor alongside its param schema, sorted by kind.
    /// This is the full catalog the UI needs to render palette + inspector.
    pub fn catalog(&self) -> Vec<(&KnotDescriptor, KnotSchema)> {
        let mut catalog: Vec<(&KnotDescriptor, KnotSchema)> =
            self.factories.values().map(|(d, _, s)| (d, s())).collect();
        catalog.sort_by_key(|(d, _)| d.kind);
        catalog
    }
}
