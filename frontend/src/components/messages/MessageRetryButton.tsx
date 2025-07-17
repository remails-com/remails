import { notifications } from "@mantine/notifications";
import { Message, MessageMetadata } from "../../types";
import { IconReload, IconX } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { useProjects } from "../../hooks/useProjects";
import { useStreams } from "../../hooks/useStreams";
import { Button } from "@mantine/core";
import { is_in_the_future } from "../../util";

function canRetryFaster(message: MessageMetadata) {
  if (message.status == "Processing" || message.status == "Accepted" || message.status == "Delivered") {
    return false;
  }

  if (message.retry_after && !is_in_the_future(message.retry_after)) {
    return false;
  }

  return true;
}

export default function MessageRetryButton({
  message,
  updateMessage,
}: {
  message: MessageMetadata;
  updateMessage: (message_id: string, update: Partial<Message>) => void;
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
      notifications.show({
        title: "Error",
        message: "Something went wrong",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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

  return (
    <Button leftSection={<IconReload />} disabled={!canRetryFaster(message)} onClick={retry}>
      Retry
    </Button>
  );
}
