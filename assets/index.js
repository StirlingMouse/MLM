document.body.addEventListener('click', e => {
        if (e.target.tagName === 'A' && e.target.getAttribute('href').startsWith('?')) {
                const link = new URLSearchParams(e.target.getAttribute('href'))
                const current = new URLSearchParams(location.search)
                const sortBy = link.get('sort_by') ?? current.get('sort_by')
                const asc = link.get('sort_by') ? link.get('asc') : current.get('asc')
                const show = link.get('show') ?? current.get('show')
                const filter = [...link.entries(), ...current.entries()].find(([key, value]) => key !== 'sort_by' && key !== 'asc' && key !== 'show')
                const combined = new URLSearchParams()
                if (sortBy) combined.set('sort_by', sortBy)
                if (asc) combined.set('asc', asc)
                if (show) combined.set('show', show)
                if (filter) combined.set(filter[0], filter[1])
                const target = new URL(e.target.href)
                target.search = combined
                e.preventDefault()
                location.href = target
        }
})

document.body.addEventListener('change', e => {
        const toggles = e.target.closest('.show_toggles')
        if (toggles) {
                setTimeout(() => {
                        const show = Array.from(toggles.querySelectorAll('input[type="checkbox"]')).filter(i => i.checked).map(i => i.value)
                        const params = new URLSearchParams(location.search)
                        params.set('show', show.join(','))
                        const target = new URL(location.href)
                        target.search = params
                        location.href = target
                })
        }
})
