import { Text } from "@mantine/core";
import { IconTrash } from "@tabler/icons-react";
import { EmailMetadata } from "../../types.ts";
import { modals } from "@mantine/modals";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails.ts";
import { errorNotification } from "../../notify.tsx";
import { MaintainerActionIcon, MaintainerButton } from "../RoleButtons.tsx";

export default function EmailDeleteButton({ email, small }: { email: EmailMetadata; small?: boolean }) {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { navigate, dispatch } = useRemails();

  if (!currentOrganization || !currentProject) {
    return null;
  }

  const deleteEmail = async () => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/emails/${email.id}`,
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
      dispatch({ type: "remove_email", emailId: email.id });
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
      onConfirm: () => deleteEmail(),
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
