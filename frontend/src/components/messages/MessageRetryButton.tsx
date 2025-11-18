import { notifications } from "@mantine/notifications";
import { Message, MessageMetadata } from "../../types";
import { IconReload } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { useProjects } from "../../hooks/useProjects";
import { is_in_the_future } from "../../util";
import { errorNotification } from "../../notify.tsx";
import { MaintainerActionIcon, MaintainerButton } from "../RoleButtons.tsx";
import { useState } from "react";

export default function MessageRetryButton({
  message,
  updateMessage,
  small,
}: {
  message: MessageMetadata;
  updateMessage: (message_id: string, update: Partial<Message>) => void;
  small?: boolean;
}) {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const [loading, setLoading] = useState(false);

  if (!currentOrganization || !currentProject) {
    return null;
  }

  const message_endpoint = `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/messages/${message.id}`;

  async function retry() {
    const res = await fetch(`${message_endpoint}/retry`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
    });
    if (res.status !== 200) {
      errorNotification("Message could not be retried");
      console.error(res);
      return;
    }

    notifications.show({
      title: "Scheduled retry",
      message: "Message will be retried soon",
      color: "blue",
      autoClose: 20000,
      icon: <IconReload size={20} />,
    });

    await new Promise((r) => setTimeout(r, 2000));

    const update = await fetch(message_endpoint);
    if (update.status !== 200) {
      errorNotification("Message could not be found");
      console.error(update);
      return;
    }
    updateMessage(message.id, await update.json());
  }

  const onClick = () => {
    setLoading(true);
    retry().finally(() => setLoading(false));
  };

  const status_retryable = !(
    message.status == "Processing" ||
    message.status == "Accepted" ||
    message.status == "Delivered"
  );
  const already_scheduled = message.retry_after && !is_in_the_future(message.retry_after);

  const can_retry = status_retryable && !already_scheduled;

  const tooltip = status_retryable
    ? already_scheduled
      ? "Message is already scheduled to retry as soon as possible"
      : "(Re-)schedule retry"
    : `Message is ${message.status.toLowerCase()}`;

  if (small) {
    return (
      <MaintainerActionIcon
        tooltip={tooltip}
        disabled={!can_retry}
        onClick={onClick}
        variant="light"
        size={30}
        loading={loading}
      >
        <IconReload />
      </MaintainerActionIcon>
    );
  } else {
    return (
      <MaintainerButton
        tooltip={tooltip}
        leftSection={<IconReload />}
        disabled={!can_retry}
        onClick={onClick}
        loading={loading}
      >
        Retry
      </MaintainerButton>
    );
  }
}
