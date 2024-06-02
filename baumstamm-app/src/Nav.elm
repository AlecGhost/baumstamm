module Nav exposing (..)

import Common exposing (palette)
import Element exposing (..)
import Element.Background as Background
import Element.Font as Font
import Element.Input exposing (button)
import Element.Region as Region
import FeatherIcons exposing (withSize)


navBar :
    { onEdit : Maybe msg
    , onSettings : Maybe msg
    , onUpload : Maybe msg
    , onNew : Maybe msg
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
        [ navIcon [] { icon = FeatherIcons.filePlus, onPress = onNew }
        , navIcon [] { icon = FeatherIcons.upload, onPress = onUpload }
        , navIcon [] { icon = FeatherIcons.edit, onPress = onEdit }
        , navIcon [ alignBottom ] { icon = FeatherIcons.settings, onPress = onSettings }
        ]


navIcon : List (Attribute msg) -> { icon : FeatherIcons.Icon, onPress : Maybe msg } -> Element msg
navIcon attributes { icon, onPress } =
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
                , Font.color palette.action
                , mouseOver [ Font.color palette.marker ]
                ]

            else
                [ Font.color palette.bg ]
    in
    el ([ centerX, Element.paddingXY 0 5 ] |> List.append attributes) <|
        button
            attrs
            { label =
                icon
                    |> withSize 40
                    |> FeatherIcons.toHtml []
                    |> Element.html
            , onPress = onPress
            }
