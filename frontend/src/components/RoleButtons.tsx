import { ActionIcon, ActionIconProps, Button, ButtonProps, Tooltip } from "@mantine/core";
import { ReactNode } from "react";
import { useOrgRole } from "../hooks/useOrganizations";

type AdditionalProps = {
  children: ReactNode;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  tooltip?: string;
} & React.ButtonHTMLAttributes<HTMLButtonElement>;
type RoleButtonProps = ButtonProps & AdditionalProps;
type RoleActionIconProps = ActionIconProps & AdditionalProps;

function RoleButton(role: "admin" | "maintainer", { children, tooltip, disabled, ...props }: RoleButtonProps) {
  const { isAdmin, isMaintainer } = useOrgRole();
  const has_access = role == "admin" ? isAdmin : isMaintainer;

  return (
    <Tooltip disabled={has_access && !tooltip} label={!has_access ? `You need ${role} rights to do this` : tooltip}>
      <Button disabled={!has_access || disabled} {...props}>
        {children}
      </Button>
    </Tooltip>
  );
}

export function AdminButton(props: RoleButtonProps) {
  return RoleButton("admin", props);
}

export function MaintainerButton(props: RoleButtonProps) {
  return RoleButton("maintainer", props);
}

function RoleActionIcon(role: "admin" | "maintainer", { children, tooltip, disabled, ...props }: RoleActionIconProps) {
  const { isAdmin, isMaintainer } = useOrgRole();
  const has_access = role == "admin" ? isAdmin : isMaintainer;

  return (
    <Tooltip disabled={has_access && !tooltip} label={!has_access ? `You need ${role} rights to do this` : tooltip}>
      <ActionIcon disabled={!has_access || disabled} {...props}>
        {children}
      </ActionIcon>
    </Tooltip>
  );
}

export function AdminActionIcon(props: RoleActionIconProps) {
  return RoleActionIcon("admin", props);
}

export function MaintainerActionIcon(props: RoleActionIconProps) {
  return RoleActionIcon("maintainer", props);
}
