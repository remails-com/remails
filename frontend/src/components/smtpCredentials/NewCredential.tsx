import {useState} from "react";
import {useCurrentOrganization} from "../../hooks/useCurrentOrganization.ts";
import {useProjects} from "../../hooks/useProjects.ts";
import {useStreams} from "../../hooks/useStreams.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {SmtpCredentialResponse} from "../../types.ts";
import {useForm} from "@mantine/form";
import {Button, Group, Modal, PasswordInput, Stack, Stepper, Textarea, TextInput} from "@mantine/core";
import {useDisclosure} from "@mantine/hooks";
import {notifications} from "@mantine/notifications";
import {IconX} from "@tabler/icons-react";

interface FormValues {
  username: string,
  description: string,
}

interface NewCredentialProps {
  opened: boolean;
  close: () => void;
}

export function NewCredential({opened, close}: NewCredentialProps) {
  const [activeStep, setActiveStep] = useState(0);
  const [visible, {toggle: toggleVisible}] = useDisclosure(false);
  const currentOrganization = useCurrentOrganization()
  const {currentProject} = useProjects();
  const {currentStream} = useStreams();
  const [newCredential, setNewCredential] = useState<SmtpCredentialResponse | null>(null);
  const {dispatch} = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      username: "",
      description: ""
    },
    validate: {
      username: (value) => (value.length < 3 ? 'Username must have at least 3 character' : null),
    }
  });

  if (!currentOrganization || !currentProject || !currentStream) {
    console.error("Cannot create SMTP credential without a selected organization. project, and stream")
    return <></>
  }

  const create = (values: FormValues) => {
    fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/smtp_credentials`, {
      method: 'POST',
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values)
    }).then(res => {
      if (res.status === 201) {
        res.json().then(newCredential => {
          console.log(newCredential)
          setNewCredential(newCredential)
          dispatch({type: "add_credential", credential: {...newCredential, cleartext_password: undefined}})
          setActiveStep(1)
        })
      } else if (res.status === 409) {
        form.setFieldError('username', 'This username already exists')
        return
      } else {
        notifications.show({
          title: 'Error',
          message: 'Something went wrong',
          color: 'red',
          autoClose: 20000,
          icon: <IconX size={20}/>,
        })
      }
    })
  }

  return (
    <>
      <Modal opened={opened} onClose={() => {
        close();
        setActiveStep(0);
        setNewCredential(null);
      }} title="Create New SMTP credential" size="lg">
        <Stepper active={activeStep} onStepClick={setActiveStep}>
          <Stepper.Step label="Create" allowStepSelect={false}>
            <form onSubmit={form.onSubmit(create)}>
              <Stack>
                <TextInput
                  label="Username"
                  description="The final username will contain parts of your organization ID to ensure global uniqueness"
                  key={form.key('username')}
                  value={form.values.username}
                  error={form.errors.username}
                  onChange={(event) => form.setFieldValue('username', event.currentTarget.value)}/>
                <Textarea
                  label="Description"
                  key={form.key('description')}
                  value={form.values.description}
                  error={form.errors.description}
                  onChange={(event) => form.setFieldValue('description', event.currentTarget.value)}/>
                <Group justify="space-between">
                  <Button onClick={close} variant="outline">Cancel</Button>
                  <Button type="submit" loading={form.submitting}>Create</Button>
                </Group>
              </Stack>
            </form>
          </Stepper.Step>
          <Stepper.Step label="Configure" allowStepSelect={false}>
            <Stack>
              <TextInput
                label="Username"
                variant="filled"
                readOnly
                value={newCredential?.username}
              />
              <PasswordInput
                label="Password"
                variant='filled'
                readOnly
                visible={visible}
                onVisibilityChange={toggleVisible}
                value={newCredential?.cleartext_password}
              />
              <Group justify='flex-end'>
                <Button onClick={close}>Done</Button>
              </Group>
            </Stack>
          </Stepper.Step>
        </Stepper>
      </Modal>
    </>
  )
}