import { ActionIcon, ActionIconProps, Button, ButtonProps, Tooltip } from "@mantine/core";
import { ReactNode } from "react";
import { useSelector } from "../hooks/useSelector";
import { useOrganizations } from "../hooks/useOrganizations";
import { Organization, User } from "../types";

type AdditionalProps = {
  children: ReactNode;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  tooltip?: string;
} & React.ButtonHTMLAttributes<HTMLButtonElement>;
type RoleButtonProps = ButtonProps & AdditionalProps;
type RoleActionIconProps = ActionIconProps & AdditionalProps;

function is_admin(user: User, currentOrganization: Organization | undefined) {
  return (
    user.org_roles.some((o) => o.org_id == currentOrganization?.id && o.role == "admin") || user.global_role == "admin"
  );
}

function is_maintainer(user: User, currentOrganization: Organization | undefined) {
  return (
    user.org_roles.some((o) => o.org_id == currentOrganization?.id && (o.role == "maintainer" || o.role == "admin")) ||
    user.global_role == "admin"
  );
}

function RoleButton(role: "admin" | "maintainer", { children, tooltip, disabled, ...props }: RoleButtonProps) {
  const user = useSelector((state) => state.user);
  const { currentOrganization } = useOrganizations();

  const access_check = role == "admin" ? is_admin : is_maintainer;
  const access_disabled = !access_check(user, currentOrganization);

  return (
    <Tooltip
      disabled={!(access_disabled || tooltip)}
      label={access_disabled ? `You need ${role} rights to do this` : tooltip}
    >
      <Button disabled={access_disabled || disabled} {...props}>
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
  const user = useSelector((state) => state.user);
  const { currentOrganization } = useOrganizations();

  const access_check = role == "admin" ? is_admin : is_maintainer;
  const access_disabled = !access_check(user, currentOrganization);

  return (
    <Tooltip
      disabled={!(access_disabled || tooltip)}
      label={access_disabled ? `You need ${role} rights to do this` : tooltip}
    >
      <ActionIcon disabled={access_disabled || disabled} {...props}>
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
