import { Button, IconAlert, WizardProvider } from '@left-curve/applets-kit'
import { createLazyFileRoute } from '@tanstack/react-router'

import {
  SignupCredentialStep,
  SignupUsernameStep,
  SignupWrapper,
} from '~/components/signup'
import { SignupMobile } from '~/components/signup/SignupMobile'

export const Route = createLazyFileRoute('/(auth)/_auth/signup')({
  component: SignupComponent,
})

function SignupComponent() {
  return (
    <div>
      <SignupMobile />
      <WizardProvider wrapper={<SignupWrapper />} persistKey="signup-form">
        <SignupCredentialStep />
        <SignupUsernameStep />
      </WizardProvider>
    </div>
  )
}
