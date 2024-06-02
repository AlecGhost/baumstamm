module Tree exposing (..)

import Connections
import Data exposing (GridItem(..), Pid, TreeData)
import Element exposing (..)
import Person


view : { treeData : TreeData, activePerson : Maybe Pid, onSelect : Pid -> msg } -> Element msg
view { treeData, activePerson, onSelect } =
    let
        viewItem : GridItem -> Element msg
        viewItem item =
            el
                [ width (px 200)
                , height (px 200)
                ]
                (case item of
                    PersonItem pid ->
                        let
                            isActive =
                                activePerson == Just pid
                        in
                        Person.view
                            { pid = pid
                            , isActive = isActive
                            , treeData = treeData
                            , onSelect = onSelect
                            }

                    ConnectionsItem connections ->
                        Connections.view connections
                )

        viewRow : List GridItem -> Element msg
        viewRow r =
            row []
                (r |> List.map viewItem)

        viewColumn : List (List GridItem) -> Element msg
        viewColumn c =
            column []
                (c |> List.map viewRow)
    in
    viewColumn treeData.grid
