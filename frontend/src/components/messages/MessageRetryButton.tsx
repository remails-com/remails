import { notifications } from "@mantine/notifications";
import { Message, MessageMetadata } from "../../types";
import { IconReload } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { useProjects } from "../../hooks/useProjects";
import { useStreams } from "../../hooks/useStreams";
import { is_in_the_future } from "../../util";
import { errorNotification } from "../../notify.tsx";
import { MaintainerActionIcon, MaintainerButton } from "../RoleButtons.tsx";

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
  const { currentStream } = useStreams();

  if (!currentOrganization || !currentProject || !currentStream) {
    return null;
  }

  const retry_endpoint = `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/messages/${message.id}/retry`;

  async function retry() {
    const res = await fetch(retry_endpoint, {
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
    const update = await res.json();
    updateMessage(message.id, update);

    notifications.show({
      title: "(Re-)scheduled retry",
      message: "Message will be retried soon",
      color: "blue",
      autoClose: 20000,
      icon: <IconReload size={20} />,
    });
  }

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
      <MaintainerActionIcon tooltip={tooltip} disabled={!can_retry} onClick={retry} variant="light" size={30}>
        <IconReload />
      </MaintainerActionIcon>
    );
  } else {
    return (
      <MaintainerButton tooltip={tooltip} leftSection={<IconReload />} disabled={!can_retry} onClick={retry}>
        Retry
      </MaintainerButton>
    );
  }
}
