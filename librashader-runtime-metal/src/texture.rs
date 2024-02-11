use icrate::Metal::MTLTexture;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;

pub type MetalTexture = Id<ProtocolObject<dyn MTLTexture>>;