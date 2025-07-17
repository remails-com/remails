import { ActionIcon, Button, Text, Tooltip } from "@mantine/core";
import { IconTrash, IconX } from "@tabler/icons-react";
import { MessageMetadata } from "../../types";
import { modals } from "@mantine/modals";
import { useOrganizations } from "../../hooks/useOrganizations";
import { useProjects } from "../../hooks/useProjects";
import { useStreams } from "../../hooks/useStreams";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails";

export default function MessageDeleteButton({ message, small }: { message: MessageMetadata; small?: boolean }) {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentStream } = useStreams();
  const { navigate, dispatch } = useRemails();

  if (!currentOrganization || !currentProject || !currentStream) {
    return null;
  }

  const deleteMessage = async () => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/messages/${message.id}`,
      {
        method: "DELETE",
      }
    );
    if (res.status === 200) {
      notifications.show({
        title: "Message deleted",
        message: "Message was deleted",
        color: "green",
      });
      dispatch({ type: "remove_message", messageId: message.id });
      navigate("projects.project.streams.stream.messages");
    } else {
      notifications.show({
        title: "Error",
        message: `Message could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
    }
  };

  const confirmDeleteCredential = () => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: <Text>Are you sure you want to delete this message?</Text>,
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteMessage(),
    });
  };

  return (
    <Tooltip label="Delete message">
      {small ? (
        <ActionIcon variant="light" onClick={confirmDeleteCredential} size={30}>
          <IconTrash />
        </ActionIcon>
      ) : (
        <Button leftSection={<IconTrash />} variant="outline" onClick={confirmDeleteCredential}>
          Delete
        </Button>
      )}
    </Tooltip>
  );
}
