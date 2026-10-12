//! Conversions between transfer objects and domain entities.

/// Converts `A` into `B`, for example a network DTO into a domain entity.
pub trait Mapper<A, B>: Send + Sync {
    /// Converts one value.
    fn map(&self, from: A) -> B;

    /// Converts many values.
    fn map_all(&self, from: impl IntoIterator<Item = A>) -> Vec<B>
    where
        Self: Sized,
    {
        from.into_iter().map(|a| self.map(a)).collect()
    }
}

/// A [`Mapper`] backed by a closure. Build it with [`mapper`].
pub struct FnMapper<F>(F);

impl<A, B, F: Fn(A) -> B + Send + Sync> Mapper<A, B> for FnMapper<F> {
    fn map(&self, from: A) -> B {
        (self.0)(from)
    }
}

/// Builds a mapper from a closure.
pub fn mapper<A, B, F: Fn(A) -> B + Send + Sync>(f: F) -> FnMapper<F> {
    FnMapper(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UserDto {
        first: String,
        last: String,
    }

    /// Closure mappers convert single values and collections.
    #[test]
    fn maps() {
        let m = mapper(|d: UserDto| format!("{} {}", d.first, d.last));
        let names = m.map_all([
            UserDto {
                first: "Ada".into(),
                last: "Lovelace".into(),
            },
            UserDto {
                first: "Alan".into(),
                last: "Turing".into(),
            },
        ]);
        assert_eq!(names, vec!["Ada Lovelace", "Alan Turing"]);
    }
}
