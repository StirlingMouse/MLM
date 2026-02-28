class ClearButton extends HTMLButtonElement {
        constructor() {
                super();
        }

        inputListener = () => {
                if (this.input.value) this.show(); else this.hide()
        }

        connectedCallback() {
                const input = this.previousElementSibling
                if (!(input instanceof HTMLInputElement)) {
                        throw Error('clear-button must be after input')
                }
                this.input = input
                if (input.value) this.show()
                input.addEventListener('change', this.inputListener)
                this.addEventListener('click', e => {
                        console.log('clear click')
                        e.preventDefault()
                        e.stopPropagation()
                        input.value = ''
                        this.hide()
                        input.form.submit()
                })
        }

        disconnectedCallback() {
                this.input?.removeEventListener('change', this.inputListener)
        }

        show() {
                this.style.display = 'inline-flex'
        }

        hide() {
                this.style.display = 'none'
        }
}
customElements.define("clear-button", ClearButton, { extends: 'button' })
