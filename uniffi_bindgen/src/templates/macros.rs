{#
// Template to receive calls into rust.
#}

{%- macro to_rs_call(func) -%}
{{ func.name() }}({% call _arg_list_rs_call(func) -%})
{%- endmacro -%}

{%- macro to_rs_call_with_prefix(arg_name, func, obj) -%}
    {{ func.name() }}(
    {% if obj.threadsafe() %}
    &*{{- arg_name }}
    {% else %}
    &mut *{{- arg_name }}.lock().unwrap()
    {% endif %}
    {% if func.arguments().len() > 0 %}, {% call _arg_list_rs_call(func) -%}{% endif -%}
)
{%- endmacro -%}

{%- macro _arg_list_rs_call(func) %}
    {%- for arg in func.arguments() %}
        {%- if arg.by_ref() %}&{% endif %}
        {{- arg.name()|lift_rs(arg.type_()) }}
        {%- if !loop.last %}, {% endif %}
    {%- endfor %}
{%- endmacro -%}

{#-
// Arglist as used in the _UniFFILib function declations.
// Note unfiltered name but type_ffi filters.
-#}
{%- macro arg_list_ffi_decl(func) %}
    {%- for arg in func.arguments() %}
        {{- arg.name() }}: {{ arg.type_()|type_ffi -}}{% if loop.last %}{% else %},{% endif %}
    {%- endfor %}
    {% if func.arguments().len() > 0 %},{% endif %} err: &mut uniffi::deps::ffi_support::ExternError,
{%- endmacro -%}

{%- macro arg_list_decl_with_prefix(prefix, meth) %}
    {{- prefix -}}
    {%- if meth.arguments().len() > 0 %}, {# whitespace #}
        {%- for arg in meth.arguments() %}
            {{- arg.name() }}: {{ arg.type_()|type_rs -}}{% if loop.last %}{% else %},{% endif %}
        {%- endfor %}
    {%- endif %}
{%- endmacro -%}

{% macro return_type_func(func) %}{% match func.ffi_func().return_type() %}{% when Some with (return_type) %}{{ return_type|type_ffi }}{%- else -%}(){%- endmatch -%}{%- endmacro -%}

{% macro ret(func) %}{% match func.return_type() %}{% when Some with (return_type) %}{{ "_retval"|lower_rs(return_type) }}{% else %}_retval{% endmatch %}{% endmacro %}

{% macro to_rs_constructor_call(obj, cons) %}
{% match cons.throws() %}
{% when Some with (e) %}
    todo!("XXX - catch_unwind like below...");
    let constructed = {{ obj.name() }}::{% call to_rs_call(cons) %}?;
    let arc = std::sync::Arc::new(constructed);
    Ok(std::sync:Arc::into_raw(arc))
{% else %}
    match std::panic::catch_unwind(|| {
        {%- if obj.threadsafe() %}
        let _new = {{ obj.name() }}::{% call to_rs_call(cons) %};
        {%- else %}
        let _new = std::sync::Mutex::new({{ obj.name() }}::{% call to_rs_call(cons) %});
        {%- endif %}
        let _arc = std::sync::Arc::new(_new);
        std::sync::Arc::into_raw(_arc)
    }) {
        Ok(ptr) => {
            *err = uniffi::deps::ffi_support::ExternError::default();
            ptr as usize
        },
        Err(e) => {
            *err = e.into();
            0 as usize /*`std::ptr::null()` is a compile error */
        }
    }
{% endmatch %}
{% endmacro %}

{% macro get_arc(obj) -%}
    {% if obj.threadsafe() %}
    let _arc = unsafe { std::sync::Arc::from_raw(ptr as *const {{ obj.name() }}) };
    {% else %}
    let _arc = unsafe { std::sync::Arc::from_raw(ptr as *const std::sync::Mutex<{{ obj.name() }}>) };
    {% endif %}
    // This arc now "owns" the reference but we need an outstanding reference still.
    std::sync::Arc::into_raw(std::sync::Arc::clone(&_arc));
{% endmacro %}


{% macro to_rs_method_call(obj, meth) -%}
{% match meth.throws() -%}
{% when Some with (e) -%}
    uniffi::deps::ffi_support::call_with_result(
        err,
        || -> Result<_, {{ e }}> {
            {% call get_arc(obj) %}
            let _retval = {{ obj.name() }}::{%- call to_rs_call_with_prefix("_arc", meth, obj) -%}?;
            Ok({% call ret(meth) %})
        },
    )
{% else -%}
    uniffi::deps::ffi_support::call_with_output(
        err,
        || {
            {% call get_arc(obj) %}
            let _retval = {{ obj.name() }}::{%- call to_rs_call_with_prefix("_arc", meth, obj) -%};
            {% call ret(meth) %}
        },
    )
{% endmatch -%}
{% endmacro -%}

{% macro to_rs_function_call(func) %}
{% match func.throws() %}
{% when Some with (e) %}
uniffi::deps::ffi_support::call_with_result(err, || -> Result<{% call return_type_func(func) %}, {{e}}> {
    let _retval = {% call to_rs_call(func) %}?;
    Ok({% call ret(func) %})
})
{% else %}
uniffi::deps::ffi_support::call_with_output(err, || {
    let _retval = {% call to_rs_call(func) %};
    {% call ret(func) %}
})
{% endmatch %}
{% endmacro %}
