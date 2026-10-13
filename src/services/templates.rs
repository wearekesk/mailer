//! Template CRUD service.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::database::entities::template;
use crate::utils::errors::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct TemplateInput {
    pub template_name: String,
    pub html_text: Option<String>,
    pub plain_text: Option<String>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
}

pub async fn list_for_brand(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<template::Model>> {
    Ok(template::Entity::find()
        .filter(template::Column::App.eq(brand_id))
        .order_by_asc(template::Column::Id)
        .all(db)
        .await?)
}

pub async fn find(db: &DatabaseConnection, id: i32) -> AppResult<Option<template::Model>> {
    Ok(template::Entity::find_by_id(id).one(db).await?)
}

pub async fn create(
    db: &DatabaseConnection,
    user_id: i32,
    brand_id: i32,
    input: TemplateInput,
) -> AppResult<i32> {
    let active = template::ActiveModel {
        user_id: Set(Some(user_id)),
        app: Set(Some(brand_id)),
        template_name: Set(Some(input.template_name)),
        html_text: Set(input.html_text),
        plain_text: Set(input.plain_text),
        from_name: Set(input.from_name),
        from_email: Set(input.from_email),
        reply_to: Set(input.reply_to),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    input: TemplateInput,
) -> AppResult<()> {
    let row = template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("template {id}")))?;
    let mut active: template::ActiveModel = row.into();
    active.template_name = Set(Some(input.template_name));
    active.html_text = Set(input.html_text);
    active.plain_text = Set(input.plain_text);
    active.from_name = Set(input.from_name);
    active.from_email = Set(input.from_email);
    active.reply_to = Set(input.reply_to);
    active.update(db).await?;
    Ok(())
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    template::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
