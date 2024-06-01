module Main exposing (..)

import Browser
import Common
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Font as Font
import Element.Input exposing (button)
import Element.Region as Region
import FeatherIcons exposing (withSize)
import File exposing (File)
import File.Select as Select
import Html exposing (Html)
import Html.Attributes as HA
import Json.Decode as Decode exposing (Value)
import PanZoom
import Rpc
import Task



-- MAIN


main : Program Value Model Msg
main =
    Browser.element { init = init, subscriptions = subscriptions, update = update, view = view }


subscriptions : Model -> Sub Msg
subscriptions _ =
    Rpc.receive (Rpc.decodeIncoming >> ReceiveRcp)



-- MODEL


type alias Flags =
    { isTauri : Bool }


decodeFlags : Value -> Flags
decodeFlags value =
    let
        flagDecoder =
            Decode.map Flags
                (Decode.field "isTauri" Decode.bool)
    in
    Result.withDefault
        { isTauri = False }
        (Decode.decodeValue flagDecoder value)


type Frame
    = TreeFrame
    | SettingsFrame


type alias Canvas =
    { width : Int, height : Int }


type alias Model =
    { flags : Flags
    , file : String
    , treeData : Maybe Common.TreeData
    , frame : Frame
    , modal : Maybe (Element Msg)
    , panzoom : PanZoom.Model Msg
    }


init : Value -> ( Model, Cmd Msg )
init flags =
    ( { flags = decodeFlags flags
      , file = ""
      , treeData = Nothing
      , frame = TreeFrame
      , modal = Nothing
      , panzoom =
            PanZoom.init
                (PanZoom.defaultConfig UpdatePanZoom)
                { scale = 1, position = { x = 100, y = 100 } }
      }
    , Cmd.none
    )



-- UPDATE


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | ToggleSettings
    | ShowModal (Element Msg)
    | HideModal
    | UpdatePanZoom PanZoom.MouseEvent
    | New
    | NoOp


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SendRpc data ->
            ( model, Rpc.encodeOutgoing data |> Rpc.send )

        ReceiveRcp (Rpc.TreeData data) ->
            ( { model | frame = TreeFrame, treeData = Just data }, Cmd.none )

        ReceiveRcp _ ->
            ( model, Cmd.none )

        SelectFile ->
            ( model, Select.file [ "application/json" ] LoadFile )

        LoadFile file ->
            ( model
            , Task.perform (Rpc.Load >> SendRpc) (File.toString file)
            )

        ToggleSettings ->
            ( { model
                | frame =
                    case model.frame of
                        TreeFrame ->
                            SettingsFrame

                        SettingsFrame ->
                            TreeFrame
              }
            , Cmd.none
            )

        ShowModal element ->
            ( { model | modal = Just element }, Cmd.none )

        HideModal ->
            ( { model | modal = Nothing }, Cmd.none )

        UpdatePanZoom event ->
            ( { model | panzoom = PanZoom.update event model.panzoom }, Cmd.none )

        New ->
            update (SendRpc Rpc.New) model

        NoOp ->
            ( model, Cmd.none )



-- VIEW


palette : { bg : Color, fg : Color, action : Color, marker : Color }
palette =
    { bg = rgb255 48 56 65
    , fg = rgb255 58 71 80
    , action = rgb255 0 173 181
    , marker = rgb255 238 238 238
    }


view : Model -> Html Msg
view model =
    Element.layoutWith
        { options =
            [ focusStyle
                { backgroundColor = Nothing
                , shadow = Nothing
                , borderColor = Just palette.marker
                }
            ]
        }
        [ Background.color palette.bg, width fill, height fill ]
    <|
        row
            [ height fill
            , width fill
            ]
            [ navBar
            , body model
            ]


navBar : Element Msg
navBar =
    column
        [ Region.navigation
        , spacing 7
        , Background.color palette.fg
        , height fill
        , width (px 80)
        ]
        [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just New }
        , navIcon [] { icon = FeatherIcons.upload, onPress = Just SelectFile }
        , navIcon []
            { icon = FeatherIcons.edit
            , onPress = Just (ShowModal <| text "Modal")
            }
        , navIcon [ alignBottom ]
            { icon = FeatherIcons.settings
            , onPress = Just ToggleSettings
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


buttonStyles : List (Attribute msg)
buttonStyles =
    [ Border.rounded 15
    , Border.width 2
    , Border.color palette.action
    , paddingXY 2 3
    , pointer
    , mouseOver [ Border.color palette.marker ]
    ]


body : Model -> Element Msg
body model =
    el
        ([ Region.mainContent
         , width fill
         , height fill
         , HA.style "overflow" "hidden" |> htmlAttribute
         ]
            |> List.append
                (case model.modal of
                    Just element ->
                        [ modal <|
                            el [ centerX, centerY, width fill, height fill ] <|
                                element
                        ]

                    Nothing ->
                        []
                )
        )
    <|
        case model.frame of
            SettingsFrame ->
                el [ centerX, centerY ] <| text "Settings"

            TreeFrame ->
                case model.treeData of
                    -- draw tree
                    Just treeData ->
                        PanZoom.view model.panzoom
                            { viewportAttributes = [ width fill, height fill ], contentAttributes = [] }
                        <|
                            text (Debug.toString treeData)

                    Nothing ->
                        -- draw start page
                        column [ centerX, centerY, spacing 10 ]
                            [ text "Start a new tree or upload an existing file."
                            , row [ spacing 20, width fill ]
                                [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just New }
                                , navIcon [] { icon = FeatherIcons.upload, onPress = Just SelectFile }
                                ]
                            ]


modal : Element Msg -> Attribute Msg
modal element =
    inFront <|
        margin 0.8
            0.8
            (el
                [ Background.color palette.fg
                , width fill
                , height fill
                , paddingXY 30 30
                , Border.rounded 15
                , inFront <|
                    button
                        [ alignTop
                        , alignRight
                        , Font.color palette.action
                        , mouseOver [ Font.color palette.marker ]
                        ]
                        { label =
                            FeatherIcons.x
                                |> withSize 40
                                |> FeatherIcons.toHtml []
                                |> Element.html
                        , onPress = Just HideModal
                        }
                ]
                element
            )


margin : Float -> Float -> Element msg -> Element msg
margin percentileX percentileY element =
    let
        portionX =
            round (2 / ((1 / percentileX) - 1))

        portionY =
            round (2 / ((1 / percentileY) - 1))
    in
    row
        [ width fill
        , height fill
        ]
        [ el [ width (fillPortion 1) ] none
        , column [ width (fillPortion portionX), height fill ]
            [ el [ height (fillPortion 1) ] none
            , el
                [ width fill
                , height (fillPortion portionY)
                ]
                element
            , el [ height (fillPortion 1) ] none
            ]
        , el [ width (fillPortion 1) ] none
        ]
