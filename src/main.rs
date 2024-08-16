use ashpd::desktop::{
    settings::{ColorScheme, Contrast, Settings},
    Color,
};
use futures::stream::{Stream, StreamExt};
use std::pin::pin;
use tokio::{join, select};

trait ResultExt {
    type Ok;
    fn fix_not_found(self) -> ashpd::Result<Option<Self::Ok>>;
}

impl<T> ResultExt for ashpd::Result<T> {
    type Ok = T;

    fn fix_not_found(self) -> ashpd::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(ashpd::Error::Portal(ashpd::PortalError::NotFound(_))) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

trait SettingsExt {
    async fn appearance(&self) -> ashpd::Result<Appearance>;
    async fn appearance_stream(&self) -> ashpd::Result<impl Stream<Item = Appearance>>;
}

impl SettingsExt for Settings<'_> {
    #[culpa::try_fn]
    async fn appearance(&self) -> ashpd::Result<Appearance> {
        let (color_scheme, accent_color, contrast) =
            join!(self.color_scheme(), self.accent_color(), self.contrast(),);
        Appearance {
            color_scheme: color_scheme.fix_not_found()?,
            accent_color: accent_color.fix_not_found()?,
            contrast: contrast.fix_not_found()?,
        }
    }

    #[culpa::try_fn]
    async fn appearance_stream(&self) -> ashpd::Result<impl Stream<Item = Appearance>> {
        let appearance = self.appearance().await?;

        let (color_scheme_stream, accent_color_stream, contrast_stream) = join!(
            self.receive_color_scheme_changed(),
            self.receive_accent_color_changed(),
            self.receive_contrast_changed(),
        );

        let (color_scheme_stream, accent_color_stream, contrast_stream) =
            (color_scheme_stream?, accent_color_stream?, contrast_stream?);

        futures::stream::iter([appearance.clone()]).chain(futures::stream::unfold(
            (
                appearance,
                color_scheme_stream,
                accent_color_stream,
                contrast_stream,
            ),
            |(
                mut appearance,
                mut color_scheme_stream,
                mut accent_color_stream,
                mut contrast_stream,
            )| async move {
                select! {
                    color_scheme = color_scheme_stream.next() => {
                        appearance.color_scheme = color_scheme;
                    }
                    accent_color = accent_color_stream.next() => {
                        appearance.accent_color = accent_color;
                    }
                    contrast = contrast_stream.next() => {
                        appearance.contrast = contrast;
                    }
                }

                Some((
                    appearance.clone(),
                    (
                        appearance,
                        color_scheme_stream,
                        accent_color_stream,
                        contrast_stream,
                    ),
                ))
            },
        ))
    }
}

serde_with::serde_conv! {
    ColorSchemeAs, Option<ColorScheme>,

    |color_scheme: &Option<ColorScheme>| match color_scheme {
        Some(ColorScheme::PreferLight) => Some("light"),
        Some(ColorScheme::PreferDark) => Some("dark"),
        Some(ColorScheme::NoPreference) | None => None,
    },

    |_: ()| -> Result<_, &'static str> {
        Err("unsupported")
    }
}

serde_with::serde_conv! {
    ContrastAs, Option<Contrast>,

    |contrast: &Option<Contrast>| match contrast {
        Some(Contrast::High) => Some("high"),
        Some(Contrast::NoPreference) | None => None,
    },

    |_: ()| -> Result<_, &'static str> {
        Err("unsupported")
    }
}

#[serde_with::serde_as]
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Appearance {
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    accent_color: Option<Color>,

    #[serde_as(as = "ColorSchemeAs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    color_scheme: Option<ColorScheme>,

    #[serde_as(as = "ContrastAs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    contrast: Option<Contrast>,
}

#[culpa::try_fn]
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let proxy = Settings::new().await?;

    let mut appearance_stream = pin!(proxy.appearance_stream().await?);
    while let Some(appearance) = appearance_stream.next().await {
        println!("{}", serde_json::to_string(&appearance)?);
    }
}
