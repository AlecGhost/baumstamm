module Person exposing (..)

import Common exposing (Settings, buttonStyles, margin, onKeyboardEvent)
import Data exposing (Person, Pid, TreeData)
import Dict
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import FeatherIcons
import Url
import Utils exposing (asList, flip, select)


reservedKeys :
    { firstName : String
    , middleNames : String
    , lastName : String
    , dateOfBirth : String
    , dateOfDeath : String
    , image : String
    }
reservedKeys =
    { firstName = "@firstName"
    , middleNames = "@middleNames"
    , lastName = "@lastName"
    , dateOfBirth = "@dateOfBirth"
    , dateOfDeath = "@dateOfDeath"
    , image = "@image"
    }


getPerson : Pid -> TreeData -> Maybe Person
getPerson pid treeData =
    treeData.persons
        |> List.filter (\person -> person.id == pid)
        |> List.head


getFirstName : Person -> Maybe String
getFirstName person =
    person.info
        |> Dict.get reservedKeys.firstName


getMiddleNames : Person -> Maybe String
getMiddleNames person =
    person.info
        |> Dict.get reservedKeys.middleNames


getLastName : Person -> Maybe String
getLastName person =
    person.info
        |> Dict.get reservedKeys.lastName


getFullName : Person -> String
getFullName person =
    case ( getFirstName person, getMiddleNames person, getLastName person ) of
        ( Just firstname, Just middleNames, Just lastName ) ->
            firstname ++ " " ++ middleNames ++ " " ++ lastName

        ( Just firstname, Nothing, Just lastName ) ->
            firstname ++ " " ++ lastName

        ( Nothing, Just middleNames, Just lastName ) ->
            middleNames ++ " " ++ lastName

        ( Just firstName, Just middleNames, Nothing ) ->
            firstName ++ " " ++ middleNames

        ( Just firstName, Nothing, Nothing ) ->
            firstName

        ( Nothing, Just middleNames, Nothing ) ->
            middleNames

        ( Nothing, Nothing, Just lastName ) ->
            lastName

        ( Nothing, Nothing, Nothing ) ->
            "?"


getNames : Person -> Bool -> List String
getNames person includeMiddleNames =
    let
        firstName =
            getFirstName person

        middleNames =
            if includeMiddleNames then
                getMiddleNames person

            else
                Nothing

        lastName =
            getLastName person

        names =
            [ firstName, middleNames, lastName ]
                |> List.filterMap identity
                |> select List.isEmpty ((::) "?") identity
    in
    names


getDateOfBirth : Person -> Maybe String
getDateOfBirth person =
    person.info
        |> Dict.get reservedKeys.dateOfBirth


getDateOfDeath : Person -> Maybe String
getDateOfDeath person =
    person.info
        |> Dict.get reservedKeys.dateOfDeath


getDates : Person -> Maybe String
getDates person =
    case ( getDateOfBirth person, getDateOfDeath person ) of
        ( Just birth, Just death ) ->
            Just <| birth ++ " - " ++ death

        ( Just birth, Nothing ) ->
            Just <| "*" ++ birth

        ( Nothing, Just death ) ->
            Just <| "†" ++ death

        ( Nothing, Nothing ) ->
            Nothing


getInfo : Person -> List ( String, String )
getInfo person =
    let
        reserved =
            [ reservedKeys.firstName
            , reservedKeys.middleNames
            , reservedKeys.lastName
            , reservedKeys.dateOfBirth
            , reservedKeys.dateOfDeath
            , reservedKeys.image
            ]
    in
    person.info
        |> Dict.toList
        |> List.filter (Tuple.first >> flip List.member reserved >> not)


getImage : Person -> Maybe String
getImage person =
    person.info
        |> Dict.get reservedKeys.image



{-
   If the image is not an https resource, but a local one,
   the asset protocol must be used
-}


encodeImageUri : String -> String
encodeImageUri uri =
    if uri |> String.startsWith "https://" then
        uri

    else
        "asset://localhost/" ++ Url.percentEncode uri


view :
    { pid : Pid
    , isActive : Bool
    , treeData : TreeData
    , onSelect : Pid -> msg
    , settings : Settings
    }
    -> Element msg
view { pid, isActive, treeData, onSelect, settings } =
    case getPerson pid treeData of
        Just person ->
            margin 0.95 1 <|
                column
                    [ width fill
                    , height fill
                    , clip
                    , Background.color settings.palette.fg
                    , Border.width 2
                    , Border.rounded 15
                    , Border.color
                        (if isActive then
                            settings.palette.marker

                         else
                            settings.palette.action
                        )
                    , mouseOver [ Border.color settings.palette.marker ]
                    , onClick <| onSelect pid
                    ]
                <|
                    el [ height (px 5) ] none
                        :: (viewProfilePicture person settings |> asList)
                        ++ (viewNames person settings |> asList)
                        ++ (viewDates person settings |> asList)

        Nothing ->
            el [ Background.color (rgb 1 0 0) ] <| text "Inconsistent data!"


viewProfilePicture : Person -> Settings -> Maybe (Element msg)
viewProfilePicture person settings =
    case ( settings.showProfilePictures, getImage person ) of
        ( True, Just img ) ->
            Just <|
                el
                    [ centerX
                    , centerY
                    , width (fill |> maximum 150)
                    , height (fillPortion 2)
                    , Background.uncropped (encodeImageUri img)
                    ]
                    none

        -- either profile pictures are not switched on it is or not present
        _ ->
            Nothing


viewNames : Person -> Settings -> Maybe (Element msg)
viewNames person settings =
    let
        nameEls =
            getNames person settings.showMiddleNames
                |> List.map text
                |> List.map
                    (el
                        [ centerX
                        , centerY
                        , width (shrink |> maximum 230)
                        ]
                    )
    in
    Just <|
        column
            [ centerX
            , centerY
            , width fill
            , height (fillPortion 1)
            ]
            nameEls


viewDates : Person -> Settings -> Maybe (Element msg)
viewDates person settings =
    case ( settings.showDates, getDates person ) of
        ( True, Just dates ) ->
            Just <|
                el
                    [ centerX
                    , centerY
                    , width fill
                    , height (fillPortion 1)
                    ]
                <|
                    el
                        [ centerX
                        , centerY
                        , width (shrink |> maximum 230)
                        ]
                    <|
                        text <|
                            dates

        _ ->
            Nothing


type alias InfoTableInput msg =
    { key : String
    , value : String
    , onKeyUpdate : String -> msg
    , onValueUpdate : String -> msg
    , onSave : msg
    , onCancel : msg
    }


viewEdit :
    { pid : Pid
    , treeData : TreeData
    , onDismiss : msg
    , infoTableInput : InfoTableInput msg
    , settings : Settings
    }
    -> Element msg
viewEdit { pid, treeData, onDismiss, infoTableInput, settings } =
    let
        heading person =
            el
                [ centerX
                , Font.size 30
                ]
            <|
                text <|
                    getFullName person

        profilePicture person =
            getImage person
                |> Maybe.map
                    (\img ->
                        el
                            [ centerX
                            , width (fill |> maximum 300)
                            , height (fill |> maximum 300)
                            , Background.uncropped img
                            ]
                            none
                    )

        infoTable person =
            table []
                { data = getInfo person
                , columns =
                    [ { header = text "Key"
                      , width = fill
                      , view =
                            \info ->
                                text <| Tuple.first info
                      }
                    , { header = text "Value"
                      , width = fill
                      , view =
                            \info ->
                                text <| Tuple.second info
                      }
                    ]
                }

        tableEdit =
            column
                [ width fill, spacing 5 ]
                [ row
                    [ width fill
                    , onKeyboardEvent <|
                        \{ key } ->
                            case key of
                                "Enter" ->
                                    Just infoTableInput.onSave

                                "Escape" ->
                                    Just infoTableInput.onCancel

                                _ ->
                                    Nothing
                    ]
                    [ Input.text
                        [ Background.color settings.palette.bg
                        ]
                        { onChange = infoTableInput.onKeyUpdate
                        , label = Input.labelHidden "Key"
                        , placeholder = Just <| Input.placeholder [] <| text "Key"
                        , text = infoTableInput.key
                        }
                    , Input.text
                        [ Background.color settings.palette.bg
                        ]
                        { onChange = infoTableInput.onValueUpdate
                        , label = Input.labelHidden "Value"
                        , placeholder = Just <| Input.placeholder [] <| text "Value"
                        , text = infoTableInput.value
                        }
                    ]
                , row [ spaceEvenly, width fill ]
                    [ el [ width fill ] <|
                        Input.button buttonStyle.primary
                            { label = el [ centerX ] <| text "Save"
                            , onPress = Just infoTableInput.onSave
                            }
                    , el [ width fill ] <|
                        Input.button buttonStyle.primary
                            { label = el [ centerX ] <| text "Cancel"
                            , onPress = Just infoTableInput.onCancel
                            }
                    ]
                ]

        okButton =
            row [ alignBottom, spaceEvenly, width fill ]
                [ el [ width fill ] <|
                    Input.button buttonStyle.primary
                        { label = el [ centerX ] <| text "Ok"
                        , onPress = Just onDismiss
                        }
                ]

        buttonStyle =
            buttonStyles settings
    in
    case getPerson pid treeData of
        Just person ->
            column
                [ width fill
                , height fill
                , spacing 5
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
                [ heading person
                , profilePicture person |> Maybe.withDefault (el [] none)
                , viewStats settings person
                , infoTable person
                , tableEdit
                , okButton
                ]

        Nothing ->
            el [ Background.color (rgb 1 0 0) ] <| text "Inconsistent data!"


editIcon : Element msg
editIcon =
    FeatherIcons.edit
        |> FeatherIcons.withSize 20
        |> FeatherIcons.toHtml []
        |> html


viewStats : Settings -> Person -> Element msg
viewStats { palette } person =
    let
        data =
            [ { label = "First name", content = getFirstName person }
            , { label = "Middle names", content = getMiddleNames person }
            , { label = "Last name", content = getLastName person }
            , { label = "*", content = getDateOfBirth person }
            , { label = "†", content = getDateOfDeath person }
            ]
    in
    table [ width fill ]
        { data = data
        , columns =
            [ { header = none
              , width = fillPortion 1
              , view =
                    \row -> text row.label
              }
            , { header = none
              , width = fillPortion 20
              , view =
                    \row -> text (row.content |> Maybe.withDefault "-")
              }
            , { header = none
              , width = fillPortion 1
              , view =
                    \_ ->
                        Input.button
                            [ pointer
                            , Font.color palette.action
                            , mouseOver
                                [ Font.color palette.marker ]
                            ]
                            { label = editIcon, onPress = Nothing }
              }
            ]
        }
