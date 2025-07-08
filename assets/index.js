document.body.addEventListener('click', e => {
        if (e.target instanceof HTMLElement && e.target.tagName === 'A' && e.target.getAttribute('href').startsWith('?')) {
                const link = new URLSearchParams(e.target.getAttribute('href'))
                const current = new URLSearchParams(location.search)
                const sortBy = link.get('sort_by') ?? current.get('sort_by')
                const asc = link.get('sort_by') ? link.get('asc') : current.get('asc')
                const show = link.get('show') ?? current.get('show')
                const from = link.get('from') ?? current.get('from')
                const page_size = link.get('page_size') ?? current.get('page_size')
                const filter = [...link.entries(), ...current.entries()].find(([key, value]) => key !== 'sort_by' && key !== 'asc' && key !== 'show' && key !== 'from' && key !== 'page_size')
                const combined = new URLSearchParams()
                if (sortBy) combined.set('sort_by', sortBy)
                if (asc) combined.set('asc', asc)
                if (show) combined.set('show', show)
                if (from) combined.set('from', from)
                if (page_size) combined.set('page_size', page_size)
                if (filter) combined.set(filter[0], filter[1])
                const target = new URL(e.target.href)
                target.search = combined
                e.preventDefault()
                location.href = target
        }
})

document.body.addEventListener('change', e => {
        const toggles = e.target.closest('.option_group')
        if (toggles) {
                setTimeout(() => {
                        const value = e.target.tagName === 'SELECT'
                                ? [e.target.value]
                                : Array.from(toggles.querySelectorAll('input[type="checkbox"],input[type="radio"]'))
                                        .filter(i => i.checked)
                                        .map(i => i.type === "radio" && i.value == "on" ? '' : i.value)
                        const params = new URLSearchParams(location.search)
                        const value_str = value.join(',')
                        if (value_str) {
                                params.set(e.target.name, value_str)
                        } else {
                                params.delete(e.target.name)
                        }
                        const target = new URL(location.href)
                        target.search = params
                        location.href = target
                })
        }
})
