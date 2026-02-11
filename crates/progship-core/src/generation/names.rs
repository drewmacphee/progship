//! Name generation utilities

use crate::components::Name;
use rand::Rng;

/// Generate a random name
pub fn generate_name(rng: &mut impl Rng) -> Name {
    let given = GIVEN_NAMES[rng.gen_range(0..GIVEN_NAMES.len())];
    let family = FAMILY_NAMES[rng.gen_range(0..FAMILY_NAMES.len())];

    Name::new(given, family)
}

// Sample name lists - would be loaded from data files in production
static GIVEN_NAMES: &[&str] = &[
    // Common English
    "James",
    "John",
    "Robert",
    "Michael",
    "William",
    "David",
    "Joseph",
    "Charles",
    "Mary",
    "Patricia",
    "Jennifer",
    "Linda",
    "Elizabeth",
    "Barbara",
    "Susan",
    "Sarah",
    // International variety
    "Wei",
    "Yuki",
    "Aisha",
    "Pavel",
    "Ingrid",
    "Carlos",
    "Fatima",
    "Kenji",
    "Olga",
    "Raj",
    "Amara",
    "Dmitri",
    "Elena",
    "Hassan",
    "Priya",
    "Sven",
    "Ming",
    "Akiko",
    "Omar",
    "Katya",
    "Diego",
    "Nadia",
    "Hiroshi",
    "Leila",
    // Sci-fi appropriate
    "Zara",
    "Orion",
    "Nova",
    "Phoenix",
    "Atlas",
    "Luna",
    "Sirius",
    "Aurora",
    "Vega",
    "Lyra",
    "Cassius",
    "Thea",
    "Juno",
    "Felix",
    "Sage",
    "River",
];

static FAMILY_NAMES: &[&str] = &[
    // Common English
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Miller",
    "Davis",
    "Wilson",
    "Taylor",
    "Anderson",
    "Thomas",
    "Jackson",
    "White",
    "Harris",
    "Martin",
    "Thompson",
    // International variety
    "Chen",
    "Nakamura",
    "Patel",
    "Ivanov",
    "Mueller",
    "Garcia",
    "Kim",
    "Okonkwo",
    "Johansson",
    "Ali",
    "Petrov",
    "Nguyen",
    "Kowalski",
    "Santos",
    "Yamamoto",
    "Singh",
    "Zhang",
    "Tanaka",
    "Hassan",
    "Volkov",
    "Rodriguez",
    "Park",
    "Sato",
    "Ahmed",
    // Compound/hyphenated
    "O'Brien",
    "Van der Berg",
    "De Silva",
    "Al-Rashid",
    "Mc'Neill",
    "St. Claire",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_name() {
        let mut rng = rand::thread_rng();
        let name = generate_name(&mut rng);

        assert!(!name.given.is_empty());
        assert!(!name.family.is_empty());
    }

    #[test]
    fn test_name_variety() {
        let mut rng = rand::thread_rng();
        let names: Vec<Name> = (0..100).map(|_| generate_name(&mut rng)).collect();

        // Check we get some variety (not all the same)
        let unique_given: std::collections::HashSet<_> = names.iter().map(|n| &n.given).collect();
        let unique_family: std::collections::HashSet<_> = names.iter().map(|n| &n.family).collect();

        assert!(unique_given.len() > 10);
        assert!(unique_family.len() > 10);
    }
}
