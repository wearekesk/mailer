// Fonts & CSS
import '@fontsource/inter/index.css';
import '@fortawesome/fontawesome-free/css/all.min.css';
import '@templatical/editor/style.css';
import './main.css';

// Core UI Libraries
import * as Flowbite from 'flowbite';
import { initFlowbite } from 'flowbite';
import ApexCharts from 'apexcharts';

// Expose globals for UI components & dynamic content
window.Flowbite = Flowbite;
window.initFlowbite = initFlowbite;
window.ApexCharts = ApexCharts;

// Initialize Flowbite interactivity immediately and on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => initFlowbite());
} else {
  initFlowbite();
}

import { init, unmount } from '@templatical/editor';
import { renderToMjml } from '@templatical/renderer';
import { convertHtmlTemplate } from '@templatical/import-html';
import mjml2html from 'mjml-browser';

/**
 * Initializes the Templatical visual email editor.
 *
 * @param {string|HTMLElement} container - Selector or DOM element for editor container
 * @param {string|object} initialContent - Raw HTML string, template JSON string, or template JSON object
 * @param {function(string, object): void} onChange - Callback invoked with (htmlString, editorInstance)
 * @returns {Promise<object>} Templatical editor instance
 */
window.initTemplatical = async function(container, initialContent, onChange) {
  try {
    const el = typeof container === 'string' ? document.querySelector(container) : container;
    if (!el) {
      throw new Error(`Templatical container not found: ${container}`);
    }

    // Safely unmount existing instance on the same container if present
    if (el.__templatical_instance) {
      try {
        el.__templatical_instance.unmount();
      } catch (e) {
        console.warn('Error unmounting previous Templatical instance:', e);
      }
      el.__templatical_instance = null;
    }

    let parsedContent = undefined;
    if (initialContent) {
      if (typeof initialContent === 'object') {
        parsedContent = initialContent;
      } else if (typeof initialContent === 'string' && initialContent.trim().length > 0) {
        const trimmed = initialContent.trim();
        if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
          try {
            parsedContent = JSON.parse(trimmed);
          } catch (e) {
            console.warn('Failed to parse content as JSON, attempting HTML import:', e);
          }
        }
        if (!parsedContent && (trimmed.includes('<') || trimmed.length > 0)) {
          try {
            parsedContent = convertHtmlTemplate(trimmed);
          } catch (e) {
            console.warn('HTML import failed, editor will start with default content:', e);
          }
        }
      }
    }

    const compileMjmlToHtml = async (mjmlStr) => {
      const res = await mjml2html(mjmlStr);
      return res && res.html ? res.html : res;
    };

    const editor = await init({
      container: el,
      content: parsedContent,
      render: {
        toMjml: async (payload) => {
          return renderToMjml(payload);
        },
        compileMjml: compileMjmlToHtml,
        toHtml: async (payload) => {
          const mjml = renderToMjml(payload);
          return await compileMjmlToHtml(mjml);
        },
      },
      onChange: async (updatedContent) => {
        if (onChange) {
          try {
            let html = '';
            try {
              html = await editor.toHtml();
            } catch (err) {
              const mjml = renderToMjml(updatedContent);
              html = (await compileMjmlToHtml(mjml)) || '';
            }
            onChange(html, editor);
          } catch (err) {
            console.error('Error generating HTML on Templatical change:', err);
          }
        }
      },
    });

    el.__templatical_instance = editor;
    return editor;
  } catch (err) {
    console.error('Failed to initialize Templatical editor:', err);
    throw err;
  }
};

window.unmountTemplatical = function(container) {
  const el = typeof container === 'string' ? document.querySelector(container) : container;
  if (el && el.__templatical_instance) {
    try {
      el.__templatical_instance.unmount();
    } catch (e) {
      console.warn('Error during Templatical unmount:', e);
    }
    el.__templatical_instance = null;
  }
};

/**
 * Auto-initialize inline Templatical editors embedded directly into forms (not in modals).
 */
export function initInlineTemplaticalEditors() {
  document.querySelectorAll('[data-templatical-editor]').forEach((wrapper) => {
    if (wrapper.dataset.templaticalReady === 'true') return;
    wrapper.dataset.templaticalReady = 'true';

    const targetInputId = wrapper.dataset.targetInput;
    const input = targetInputId ? document.getElementById(targetInputId) : null;
    const initialContent = input ? input.value : '';

    const container = wrapper.querySelector('.templatical-inline-container');
    if (!container) return;

    window.initTemplatical(container, initialContent, (html) => {
      if (input) {
        input.value = html;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      }
    }).catch(err => {
      console.error('Failed to mount inline Templatical editor:', err);
    });
  });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => initInlineTemplaticalEditors());
} else {
  initInlineTemplaticalEditors();
}
