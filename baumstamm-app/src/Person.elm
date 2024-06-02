module Person exposing (..)

import Common exposing (margin, palette)
import Data exposing (Person, Pid, TreeData)
import Dict
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Utils exposing (select)


getPerson : Pid -> TreeData -> Maybe Person
getPerson pid treeData =
    treeData.persons
        |> List.filter (\person -> person.id == pid)
        |> List.head


getFirstName : Person -> Maybe String
getFirstName person =
    person.info
        |> Dict.get "@firstName"


getMiddleNames : Person -> Maybe String
getMiddleNames person =
    person.info
        |> Dict.get "@middleNames"


getLastName : Person -> Maybe String
getLastName person =
    person.info
        |> Dict.get "@lastName"


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
