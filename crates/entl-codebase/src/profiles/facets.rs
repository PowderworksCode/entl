use crate::{LanguageFacet, LanguageFacetRegistration};

pub static COMPONENT_HOST: LanguageFacet = LanguageFacet {
    id: "component-host",
    description: "source can construct reusable user-interface components",
};

pub static STRUCTURED_CODE: LanguageFacet = LanguageFacet {
    id: "structured-code",
    description: "source expresses nested executable code structure",
};

pub static STYLE_HOST: LanguageFacet = LanguageFacet {
    id: "style-host",
    description: "source can contain presentation style declarations or values",
};

crate::profiles::registry::submit! { LanguageFacetRegistration(&COMPONENT_HOST) }
crate::profiles::registry::submit! { LanguageFacetRegistration(&STRUCTURED_CODE) }
crate::profiles::registry::submit! { LanguageFacetRegistration(&STYLE_HOST) }
