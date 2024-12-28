module Nav exposing (..)

import Common exposing (Settings)
import Element exposing (..)
import Element.Background as Background
import Element.Font as Font
import Element.Input exposing (button)
import Element.Region as Region
import FeatherIcons exposing (withSize)
import Html.Attributes


navBar :
    { onEdit : Maybe msg
    , onSettings : Maybe msg
    , onUpload : Maybe msg
    , onDownload : Maybe msg
    , onNew : Maybe msg
    , settings : Settings
    }
    -> Element msg
navBar { onEdit, onUpload, onDownload, onSettings, onNew, settings } =
    column
        [ Region.navigation
        , spacing 7
        , Background.color settings.palette.fg
        , height fill
        , width (px 80)
        ]
        [ navIcon []
            { icon = FeatherIcons.filePlus
            , onPress = onNew
            , settings = settings
            }
        , navIcon []
            { icon = FeatherIcons.upload
            , onPress = onUpload
            , settings = settings
            }
        , navIcon [ Html.Attributes.download "tree.json" |> htmlAttribute ]
            { icon = FeatherIcons.download
            , onPress = onDownload
            , settings = settings
            }
        , navIcon []
            { icon = FeatherIcons.edit
            , onPress = onEdit
            , settings = settings
            }
        , navIcon [ alignBottom ]
            { icon = FeatherIcons.settings
            , onPress = onSettings
            , settings = settings
            }
        ]


navIcon :
    List (Attribute msg)
    ->
        { icon : FeatherIcons.Icon
        , onPress : Maybe msg
        , settings : Settings
        }
    -> Element msg
navIcon attributes { icon, onPress, settings } =
    let
        active =
            case onPress of
                Just _ ->
                    True

                Nothing ->
                    False

        attrs =
            if active then
                [ pointer
                , Font.color settings.palette.action
                , mouseOver [ Font.color settings.palette.marker ]
                ]

            else
                [ Font.color settings.palette.bg ]
    in
    el ([ centerX, paddingXY 0 5 ] |> List.append attributes) <|
        button
            attrs
            { label =
                icon
                    |> withSize 40
                    |> FeatherIcons.toHtml []
                    |> html
            , onPress = onPress
            }
