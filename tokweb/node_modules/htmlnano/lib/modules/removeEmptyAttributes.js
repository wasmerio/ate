'use strict';

Object.defineProperty(exports, "__esModule", {
    value: true
});
exports.default = removeEmptyAttributes;
// Source: https://www.w3.org/TR/html4/sgml/dtd.html#events (Generic Attributes)
var safeToRemoveAttrs = ['id', 'class', 'style', 'title', 'lang', 'dir', 'onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmouseover', 'onmousemove', 'onmouseout', 'onkeypress', 'onkeydown', 'onkeyup'];

/** Removes empty attributes */
function removeEmptyAttributes(tree) {
    tree.walk(function (node) {
        if (!node.attrs) {
            return node;
        }

        safeToRemoveAttrs.forEach(function (safeToRemoveAttr) {
            var attrValue = node.attrs[safeToRemoveAttr];
            if (attrValue === '' || (attrValue || '').match(/^\s+$/)) {
                delete node.attrs[safeToRemoveAttr];
            }
        });

        return node;
    });

    return tree;
}