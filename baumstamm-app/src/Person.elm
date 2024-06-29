module Person exposing (..)

import Common exposing (buttonStyles, margin, onKeyboardEvent, palette)
import Data exposing (Person, Pid, TreeData)
import Dict
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import Utils exposing (flip, select)


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
        |> Dict.get "@image"


view :
    { pid : Pid
    , isActive : Bool
    , treeData : TreeData
    , onSelect : Pid -> msg
    }
    -> Element msg
view { pid, isActive, treeData, onSelect } =
    case getPerson pid treeData of
        Just person ->
            margin 0.95 1 <|
                column
                    [ width fill
                    , height fill
                    , Background.color palette.fg
                    , Border.width 2
                    , Border.rounded 15
                    , Border.color
                        (if isActive then
                            palette.marker

                         else
                            palette.action
                        )
                    , mouseOver [ Border.color palette.marker ]
                    , onClick <| onSelect pid
                    ]
                    (let
                        firstName =
                            getFirstName person

                        middleNames =
                            getMiddleNames person

                        lastName =
                            getLastName person

                        names =
                            [ firstName, middleNames, lastName ]
                                |> List.filterMap identity
                                |> select List.isEmpty ((::) "?") identity
                     in
                     names
                        |> List.map text
                        |> List.map
                            (el
                                [ centerX, centerY ]
                            )
                    )

        Nothing ->
            el [ Background.color (rgb 1 0 0) ] <| text "Inconsistent data!"


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
    }
    -> Element msg
viewEdit { pid, treeData, onDismiss, infoTableInput } =
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
            case getImage person of
                Just img ->
                    [ el
                        [ centerX
                        , width (fill |> maximum 300)
                        , height (fill |> maximum 300)
                        , Background.uncropped img
                        ]
                        none
                    ]

                Nothing ->
                    []

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
                        [ Background.color palette.bg
                        ]
                        { onChange = infoTableInput.onKeyUpdate
                        , label = Input.labelHidden "Key"
                        , placeholder = Just <| Input.placeholder [] <| text "Key"
                        , text = infoTableInput.key
                        }
                    , Input.text
                        [ Background.color palette.bg
                        ]
                        { onChange = infoTableInput.onValueUpdate
                        , label = Input.labelHidden "Value"
                        , placeholder = Just <| Input.placeholder [] <| text "Value"
                        , text = infoTableInput.value
                        }
                    ]
                , row [ spaceEvenly, width fill ]
                    [ el [ width fill ] <|
                        Input.button buttonStyles.primary
                            { label = el [ centerX ] <| text "Save"
                            , onPress = Just infoTableInput.onSave
                            }
                    , el [ width fill ] <|
                        Input.button buttonStyles.primary
                            { label = el [ centerX ] <| text "Cancel"
                            , onPress = Just infoTableInput.onCancel
                            }
                    ]
                ]

        okButton =
            row [ alignBottom, spaceEvenly, width fill ]
                [ el [ width fill ] <|
                    Input.button buttonStyles.primary
                        { label = el [ centerX ] <| text "Ok"
                        , onPress = Just onDismiss
                        }
                ]
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
                heading person
                    :: profilePicture person
                    ++ [ infoTable person
                       , tableEdit
                       , okButton
                       ]

        Nothing ->
            el [ Background.color (rgb 1 0 0) ] <| text "Inconsistent data!"
