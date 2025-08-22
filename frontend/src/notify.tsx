import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";

export function errorNotification(message: string): void {
  notifications.show({
    title: "Error",
    message: message,
    color: "red",
    autoClose: 20000,
    icon: <IconX size={20} />,
  });
}
