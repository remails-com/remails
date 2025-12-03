import { Text } from "@mantine/core";
import { IconTrash } from "@tabler/icons-react";
import { MessageMetadata } from "../../types";
import { modals } from "@mantine/modals";
import { useOrganizations } from "../../hooks/useOrganizations";
import { useProjects } from "../../hooks/useProjects";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails";
import { errorNotification } from "../../notify.tsx";
import { MaintainerActionIcon, MaintainerButton } from "../RoleButtons.tsx";

export default function MessageDeleteButton({ message, small }: { message: MessageMetadata; small?: boolean }) {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { navigate, dispatch } = useRemails();

  if (!currentOrganization || !currentProject) {
    return null;
  }

  const deleteMessage = async () => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/emails/${message.id}`,
      {
        method: "DELETE",
      }
    );
    if (res.status === 200) {
      notifications.show({
        title: "Email deleted",
        message: "Email was deleted",
        color: "green",
      });
      navigate("projects.project.emails");
      dispatch({ type: "remove_message", messageId: message.id });
    } else {
      errorNotification("Email could not be deleted");
      console.error(res);
    }
  };

  const confirmDeleteCredential = () => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: <Text>Are you sure you want to delete this email?</Text>,
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteMessage(),
    });
  };

  if (small) {
    return (
      <MaintainerActionIcon tooltip="Delete email" variant="light" onClick={confirmDeleteCredential} size={30}>
        <IconTrash />
      </MaintainerActionIcon>
    );
  } else {
    return (
      <MaintainerButton
        tooltip="Delete email"
        leftSection={<IconTrash />}
        variant="outline"
        onClick={confirmDeleteCredential}
      >
        Delete
      </MaintainerButton>
    );
  }
}
