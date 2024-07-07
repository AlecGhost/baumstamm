module Settings exposing (..)

import Common exposing (Settings, onKeyboardEvent)
import Element exposing (..)
import Element.Font as Font
import Element.Input as Input exposing (button, checkbox)
import FeatherIcons


checkBoxIcon : Bool -> Element msg
checkBoxIcon isChecked =
    (if isChecked then
        FeatherIcons.checkSquare

     else
        FeatherIcons.square
    )
        |> FeatherIcons.toHtml []
        |> html


resetIcon : Element msg
resetIcon =
    FeatherIcons.xSquare
        |> FeatherIcons.toHtml []
        |> html


view :
    { settings : Settings
    , onUpdate : Settings -> msg
    , onReset : msg
    , onDismiss : msg
    }
    -> Element msg
view { settings, onUpdate, onReset, onDismiss } =
    el
        [ width fill
        , height fill
        , onKeyboardEvent
            (\{ key } ->
                case key of
                    "Escape" ->
                        Just onDismiss

                    _ ->
                        Nothing
            )
        ]
    <|
        column
            [ centerX, centerY ]
        <|
            [ checkbox []
                { onChange = \value -> onUpdate { settings | showProfilePictures = value }
                , checked = settings.showProfilePictures
                , label = Input.labelRight [] <| text "Show profile pictures"
                , icon = checkBoxIcon
                }
            , checkbox []
                { onChange = \value -> onUpdate { settings | showMiddleNames = value }
                , checked = settings.showMiddleNames
                , label = Input.labelRight [] <| text "Show middle names"
                , icon = checkBoxIcon
                }
            , row []
                [ button [ Font.color (rgb 1 0 0) ]
                    { onPress = Just onReset
                    , label = resetIcon
                    }
                , text " Reset to default settings"
                ]
            ]
