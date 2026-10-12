//! Plain Rust business rules: typed identity, validation and a use case.
use bevy_wings::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct Note {
    id: Id<Note>,
    title: String,
}
impl DomainEntity for Note {
    fn id(&self) -> Id<Note> {
        self.id
    }
}

fn main() -> DomainResult<()> {
    let validator = Validator::<String>::new()
        .rule(
            "title",
            |title| rules::not_blank(title),
            "A title is required",
        )
        .rule(
            "title",
            |title| rules::length_between(title, 1, 40),
            "Use at most 40 characters",
        );
    println!("Invalid title: {:?}", validator.errors(&" ".into()));
    let create = use_case(move |title: String| {
        validator.validate(&title)?;
        Ok(Note {
            id: Id::new(1),
            title,
        })
    });
    println!("Created: {:?}", create.execute("Ship Wings".into())?);
    Ok(())
}
