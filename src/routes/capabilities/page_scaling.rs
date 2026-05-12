use winprint::ticket::document::OwnedName;
use winprint::ticket::document::ParameterInit;
use winprint::ticket::document::PrintFeatureOption;
use winprint::ticket::document::NS_PSK;
use winprint::ticket::FeatureOptionPack;
use winprint::ticket::FeatureOptionPackWithPredefined;
use winprint::ticket::PredefinedName;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
/// Represents a predefined page scaling option.
pub enum PredefinedPageScaling {
    /// Specifies the page scaling should not be specified.
    None,
    /// Specifies the page scaling should fit the paper size.
    Fit,
    /// Specifies the page scaling should fill the paper size.
    Fill,
}

impl PredefinedName for PredefinedPageScaling {
    /// Get predefined media name from the given name.
    fn from_name(name: &OwnedName) -> Option<Self> {
        if name.namespace_ref()
            == Some("http://schemas.microsoft.com/windows/2018/04/printing/printschemakeywords/Ipp")
        {
            match name.local_name.as_str() {
                "None" => Some(PredefinedPageScaling::None),
                "Fit" => Some(PredefinedPageScaling::Fit),
                "Fill" => Some(PredefinedPageScaling::Fill),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
#[doc = concat!("Represents a feature option pack as [`PageScaling`].")]
pub struct PageScaling {
    /// The option of the feature.
    option: PrintFeatureOption,
    /// The parameters that is used by the option.
    parameters: Vec<ParameterInit>,
}

impl FeatureOptionPack for PageScaling {
    fn new(option: PrintFeatureOption, parameters: Vec<ParameterInit>) -> Self {
        Self { option, parameters }
    }

    fn feature_name() -> OwnedName {
        OwnedName::qualified("PageScaling", NS_PSK, Some("psk"))
    }

    fn option(&self) -> &PrintFeatureOption {
        &self.option
    }

    fn option_mut(&mut self) -> &mut PrintFeatureOption {
        &mut self.option
    }

    fn parameters(&self) -> &[ParameterInit] {
        &self.parameters
    }

    fn parameters_mut(&mut self) -> &mut Vec<ParameterInit> {
        &mut self.parameters
    }

    fn into_option_with_parameters(self) -> (PrintFeatureOption, Vec<ParameterInit>) {
        (self.option, self.parameters)
    }
}

impl FeatureOptionPackWithPredefined for PageScaling {
    type PredefinedName = PredefinedPageScaling;
}
