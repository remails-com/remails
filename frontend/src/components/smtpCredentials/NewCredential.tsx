import { useState } from "react";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useStreams } from "../../hooks/useStreams.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { SmtpCredentialResponse } from "../../types.ts";
import { useForm } from "@mantine/form";
import { Alert, Button, Code, Group, Modal, Stack, Text, Stepper, Textarea, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconInfoCircle, IconX } from "@tabler/icons-react";
import { CopyableCode } from "../CopyableCode.tsx";

interface FormValues {
  username: string;
  description: string;
}

interface NewCredentialProps {
  opened: boolean;
  close: () => void;
}

export function NewCredential({ opened, close }: NewCredentialProps) {
  const [activeStep, setActiveStep] = useState(0);
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentStream } = useStreams();
  const [newCredential, setNewCredential] = useState<SmtpCredentialResponse | null>(null);
  const { dispatch } = useRemails();

  const form = useForm<FormValues>({
    validateInputOnBlur: true,
    initialValues: {
      username: "",
      description: "",
    },
    validate: {
      username: (value) =>
        value.match(/^[a-zA-Z0-9_-]{2,128}$/)
          ? null
          : "Username must only contain alphanumeric characters or underscores and dashes and must be between 2 and 128 characters long",
    },
  });

  if (!currentOrganization || !currentProject || !currentStream) {
    console.error("Cannot create SMTP credential without a selected organization. project, and stream");
    return <></>;
  }

  const create = async (values: FormValues) => {
    console.log("match", values.username.match(/^[a-zA-Z0-9_-]{2,128}$/));

    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/smtp_credentials`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(values),
      }
    );
    if (res.status === 409) {
      form.setFieldError("username", "This username already exists");
      return;
    } else if (res.status !== 201) {
      notifications.show({
        title: "Error",
        message: "Something went wrong",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      return;
    }
    const newCredential = await res.json();
    setNewCredential(newCredential);
    dispatch({ type: "add_credential", credential: { ...newCredential, cleartext_password: undefined } });
    setActiveStep(1);
  };

  return (
    <>
      <Modal
        opened={opened}
        onClose={activeStep === 0 ? close : () => { }}
        title="Create new SMTP credential"
        size="lg"
        withCloseButton={activeStep === 0}
      >
        <Stepper active={activeStep} onStepClick={setActiveStep}>
          <Stepper.Step label="Create" allowStepSelect={false}>
            <form onSubmit={form.onSubmit(create)}>
              <Stack>
                <TextInput
                  data-autofocus
                  label="Username"
                  description="The final username will contain parts of your organization ID to ensure global uniqueness"
                  key={form.key("username")}
                  value={form.values.username}
                  error={form.errors.username}
                  onChange={(event) => form.setFieldValue("username", event.currentTarget.value)}
                />
                <Textarea
                  label="Description"
                  key={form.key("description")}
                  value={form.values.description}
                  error={form.errors.description}
                  onChange={(event) => form.setFieldValue("description", event.currentTarget.value)}
                />
                <Group justify="space-between">
                  <Button onClick={close} variant="outline">
                    Cancel
                  </Button>
                  <Button type="submit" loading={form.submitting}>
                    Create
                  </Button>
                </Group>
              </Stack>
            </form>
          </Stepper.Step>
          <Stepper.Step label="Configure" allowStepSelect={false}>
            <Stack>
              <CopyableCode label="Username">{newCredential?.username ?? ""}</CopyableCode>
              <CopyableCode label="Password">{newCredential?.cleartext_password ?? ""}</CopyableCode>
              <Alert variant="light" color="red" title="Save this password somewhere safe!" icon={<IconInfoCircle />}>
                This password will only be shown once. After you closed this window, we cannot show it again. If you
                lose it, you can simply create a new credential and delete the old one, if necessary.
              </Alert>
              <Group justify="flex-end">
                <Button
                  onClick={() => {
                    setActiveStep(0);
                    setNewCredential(null);
                    close();
                  }}
                >
                  Done
                </Button>
              </Group>
            </Stack>
          </Stepper.Step>
        </Stepper>
      </Modal>
    </>
  );
}
