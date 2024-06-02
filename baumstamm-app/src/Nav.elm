module Nav exposing (..)

import Common exposing (palette)
import Element exposing (..)
import Element.Background as Background
import Element.Font as Font
import Element.Input exposing (button)
import Element.Region as Region
import FeatherIcons exposing (withSize)


navBar :
    { onEdit : msg
    , onSettings : msg
    , onUpload : msg
    , onNew : msg
    }
    -> Element msg
navBar { onEdit, onUpload, onSettings, onNew } =
    column
        [ Region.navigation
        , spacing 7
        , Background.color palette.fg
        , height fill
        , width (px 80)
        ]
        [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just onNew }
        , navIcon [] { icon = FeatherIcons.upload, onPress = Just onUpload }
        , navIcon []
            { icon = FeatherIcons.edit
            , onPress = Just onEdit
            }
        , navIcon [ alignBottom ]
            { icon = FeatherIcons.settings
            , onPress = Just onSettings
            }
        ]


navIcon : List (Attribute msg) -> { icon : FeatherIcons.Icon, onPress : Maybe msg } -> Element msg
navIcon attributes { icon, onPress } =
    el ([ centerX, Element.paddingXY 0 5 ] |> List.append attributes) <|
        button
            [ pointer
            , Font.color palette.action
            , mouseOver [ Font.color palette.marker ]
            ]
            { label =
                icon
                    |> withSize 40
                    |> FeatherIcons.toHtml []
                    |> Element.html
            , onPress = onPress
            }
