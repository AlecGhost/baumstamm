import { PanelLeftClose, PanelLeftOpen, Settings } from 'lucide-react';
import {
    Sidebar as Bar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupContent,
    SidebarGroupLabel,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
} from './components/ui/sidebar';

type Props = { isOpen: boolean, toggle: () => void };

export function Sidebar(props: Props) {
    return (
        <Bar collapsible='icon' variant='floating'>
            <SidebarContent>
                <SidebarGroup>
                    <SidebarGroupContent>
                        <SidebarMenu>
                            <SidebarMenuItem key="1">
                                <SidebarMenuButton asChild onClick={props.toggle}>
                                    <a href="#">
                                        {props.isOpen ? <PanelLeftClose /> : <PanelLeftOpen />}
                                        {props.isOpen ? <span>Close Sidebar</span> : <div />}
                                    </a>
                                </SidebarMenuButton>
                            </SidebarMenuItem>
                        </SidebarMenu>
                    </SidebarGroupContent>
                </SidebarGroup>
                <SidebarGroup>
                    <SidebarGroupLabel>Application</SidebarGroupLabel>
                    <SidebarGroupContent>
                        <SidebarMenu>
                            <SidebarMenuItem key="1">
                                <SidebarMenuButton asChild>
                                    <a href="#">
                                        <Settings />
                                        <span>Settings</span>
                                    </a>
                                </SidebarMenuButton>
                            </SidebarMenuItem>
                        </SidebarMenu>
                    </SidebarGroupContent>
                </SidebarGroup>
            </SidebarContent>
        </Bar>
    )
}
