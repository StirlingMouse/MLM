document.body.addEventListener('submit', e => {
        // @ts-ignore
        const form = e.target.form || e.target
        if (!(form instanceof HTMLFormElement)) return

        if (form.getAttribute('inline') !== null) {
                e.preventDefault()
                const data = new URLSearchParams();
                // @ts-ignore
                for (const pair of new FormData(form)) {
                        data.append(pair[0], pair[1]);
                }
                if (e.submitter && e.submitter instanceof HTMLButtonElement && e.submitter.name) {
                        data.append(e.submitter.name, e.submitter.value)
                }
                /** @type {HTMLTemplateElement} */
                const optimistic = e.submitter.querySelector('template.optimistic') ?? form.querySelector('template.optimistic')
                if (optimistic) {
                        e.submitter.replaceChildren(optimistic.content.cloneNode(true))
                }

                fetch(location.href, {
                        method: form.method,
                        body: data,
                })
                        .then(async r => {
                                /** @type {HTMLTemplateElement} */
                                const final = e.submitter.querySelector('template.final') ?? form.querySelector('template.final')
                                if (final && r.ok) {
                                        e.submitter.replaceChildren(final.content.cloneNode(true))
                                } else {
                                        const html = await r.text()
                                        const parser = new DOMParser()
                                        const newDocument = parser.parseFromString(html, 'text/html')
                                        const main = document.querySelector('main')
                                        main.replaceWith(newDocument.querySelector('main'))
                                }
                        })
        }
})

document.body.addEventListener('formdata', e => {
        const formData = e.formData;
        const show = formData.getAll('show')
        if (show.length > 1) formData.set('show', show.join(','))
        const query = formData.get('query')
        if (query === '') formData.delete('query')
})

document.body.addEventListener('click', e => {
        if (e.target instanceof HTMLButtonElement) {
                if (e.target.dataset.prompt) {
                        if (!confirm(e.target.dataset.prompt)) {
                                e.preventDefault()
                        }
                }
        }
        if (e.target instanceof HTMLAnchorElement && e.target.getAttribute('href').startsWith('?')) {
                const link = new URLSearchParams(e.target.getAttribute('href'))
                const current = new URLSearchParams(location.search)
                const sortBy = link.get('sort_by') ?? current.get('sort_by')
                const asc = link.get('sort_by') ? link.get('asc') : current.get('asc')
                const show = link.get('show') ?? current.get('show')
                const from = link.get('from') ?? current.get('from')
                const page_size = link.get('page_size') ?? current.get('page_size')
                // @ts-ignore
                const filters = [...link.entries(), ...current.entries()]
                        .filter(([key,]) => key !== 'sort_by' && key !== 'asc' && key !== 'show' && key !== 'from' && key !== 'page_size')
                const combined = new URLSearchParams()
                if (sortBy) combined.set('sort_by', sortBy)
                if (asc) combined.set('asc', asc)
                if (show) combined.set('show', show)
                if (from) combined.set('from', from)
                if (page_size) combined.set('page_size', page_size)
                if (e.shiftKey) {
                        for (const filter of filters) combined.set(filter[0], filter[1])
                } else {
                        const filter = filters[0]
                        if (filter) combined.set(filter[0], filter[1])
                }
                const target = new URL(e.target.href)
                target.search = combined.toString()
                e.preventDefault()
                location.href = target.toString()
        }
})

document.body.addEventListener('change', e => {
        // @ts-ignore
        const toggles = e.target.closest('.option_group')
        if (toggles?.classList.contains('query')) {
                setTimeout(() => {
                        // @ts-ignore
                        const value = e.target.tagName === 'SELECT'
                                // @ts-ignore
                                ? [e.target.value]
                                : Array.from(toggles.querySelectorAll('input[type="checkbox"],input[type="radio"]'))
                                        .filter(i => i.checked)
                                        .map(i => i.type === "radio" && i.value == "on" ? '' : i.value)
                        const params = new URLSearchParams(location.search)
                        const value_str = value.join(',')
                        if (value_str) {
                                // @ts-ignore
                                params.set(e.target.name, value_str)
                        } else {
                                // @ts-ignore
                                params.delete(e.target.name)
                        }
                        const target = new URL(location.href)
                        target.search = params.toString()
                        location.href = target.toString()
                })
                return
        }
        // @ts-ignore
        if (e.target.type === 'checkbox') {
                // @ts-ignore
                const table = e.target.closest('.table,.table2')
                if (table) {
                        // @ts-ignore
                        const name = e.target.name
                        if (name.endsWith('_all')) {
                                for (const checkbox of Array.from(table.querySelectorAll(`input[type="checkbox"][name="${name.slice(0, -4)}"]`))) {
                                        // @ts-ignore
                                        checkbox.checked = e.target.checked
                                }
                                const actions = document.querySelector(`.actions.actions_${name.slice(0, -4)}`)
                                // @ts-ignore
                                actions.style.display = e.target.checked ? 'flex' : 'none';
                        } else {
                                const all = table.querySelector(`input[type="checkbox"][name="${name}_all"]`)
                                if (all) {
                                        const actions = document.querySelector(`.actions.actions_${name}`)
                                        const checkboxes = Array.from(table.querySelectorAll(`input[type="checkbox"][name="${name}"]`))
                                        const allNotChecked = checkboxes.every(c => !c.checked)
                                        if (allNotChecked) {
                                                all.checked = false
                                                all.indeterminate = false
                                                // @ts-ignore
                                                if (actions) actions.style.display = 'none';
                                                return
                                        }

                                        // @ts-ignore
                                        if (actions) actions.style.display = 'flex';

                                        const allChecked = checkboxes.every(c => c.checked)
                                        if (allChecked) {
                                                all.checked = true
                                                all.indeterminate = false
                                                return
                                        }
                                        all.checked = false
                                        all.indeterminate = true
                                }
                        }
                }
        }
})
