export default defineAppConfig({
    ui: {
        colors: {
            primary: 'sky',
            neutral: 'zinc'
        },
        button: {
            slots: {
                base: 'rounded-none font-mono text-[11px] uppercase tracking-[0.16em] transition-all duration-300'
            }
        },
        input: {
            slots: {
                base: 'rounded-none bg-ink-900 ring-line focus-visible:ring-acid/60 placeholder:text-fg-faint'
            }
        },
        select: {
            slots: {
                base: 'rounded-none bg-ink-900 ring-line focus-visible:ring-acid/60',
                content: 'rounded-none bg-ink-800 ring-line',
                item: 'rounded-none'
            }
        },
        selectMenu: {
            slots: {
                base: 'rounded-none bg-ink-900 ring-line',
                content: 'rounded-none bg-ink-800 ring-line',
                item: 'rounded-none'
            }
        },
        badge: {
            slots: {
                base: 'rounded-none font-mono uppercase tracking-[0.14em]'
            }
        },
        modal: {
            slots: {
                overlay: 'bg-ink-900/85 backdrop-blur-[2px]',
                content: 'rounded-none ring-0 border border-line bg-ink-800',
                header: 'border-b border-line px-6 py-4',
                title: 'font-unbounded text-[13px] uppercase tracking-[0.16em]',
                body: 'px-6 py-6',
                footer: 'border-t border-line px-6 py-4'
            }
        },
        popover: {
            slots: {
                content: 'rounded-none border border-line bg-ink-800 ring-0'
            }
        },
        toast: {
            slots: {
                root: 'rounded-none ring-line bg-ink-800',
                title: 'font-medium'
            }
        }
    }
})
