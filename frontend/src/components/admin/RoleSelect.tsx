import { Select } from "@mantine/core";
import { Role } from "../../types.ts";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";

interface RoleSelectProps {
  id: string;
  role: Role | null;
}

export default function RoleSelect({ id, role }: RoleSelectProps) {
  const { dispatch } = useRemails();
  const {
    state: { user },
  } = useRemails();

  const updateRole = async (role: Role | null) => {
    const res = await fetch(`/api/api_user/${id}/role`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(role),
    });
    if (res.ok) {
      dispatch({ type: "set_api_user_role", user_id: id, role });
    } else {
      notifications.show({
        title: "Error",
        message: "Could not update global user role",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
    }
  };

  return (
    <Select
      placeholder="none"
      size="xs"
      disabled={id === user?.id}
      data={["admin"] as Role[]}
      clearable
      value={role}
      onChange={async (value) => {
        await updateRole(value as Role);
      }}
    />
  );
}
