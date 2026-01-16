# CALIBER AWS Module Outputs

output "alb_dns_name" {
  description = "DNS name of the Application Load Balancer"
  value       = aws_lb.caliber.dns_name
}

output "alb_zone_id" {
  description = "Zone ID of the Application Load Balancer (for Route53 alias)"
  value       = aws_lb.caliber.zone_id
}

output "alb_arn" {
  description = "ARN of the Application Load Balancer"
  value       = aws_lb.caliber.arn
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster"
  value       = aws_ecs_cluster.caliber.name
}

output "ecs_cluster_arn" {
  description = "ARN of the ECS cluster"
  value       = aws_ecs_cluster.caliber.arn
}

output "ecs_service_name" {
  description = "Name of the ECS service"
  value       = aws_ecs_service.caliber.name
}

output "rds_endpoint" {
  description = "RDS instance endpoint"
  value       = aws_db_instance.caliber.endpoint
}

output "rds_address" {
  description = "RDS instance address"
  value       = aws_db_instance.caliber.address
}

output "rds_port" {
  description = "RDS instance port"
  value       = aws_db_instance.caliber.port
}

output "db_password_secret_arn" {
  description = "ARN of the database password secret"
  value       = aws_secretsmanager_secret.db_password.arn
}

output "jwt_secret_arn" {
  description = "ARN of the JWT secret"
  value       = aws_secretsmanager_secret.jwt_secret.arn
}

output "cloudwatch_log_group" {
  description = "CloudWatch log group name"
  value       = aws_cloudwatch_log_group.caliber.name
}

output "security_group_alb_id" {
  description = "Security group ID for ALB"
  value       = aws_security_group.alb.id
}

output "security_group_ecs_id" {
  description = "Security group ID for ECS tasks"
  value       = aws_security_group.ecs.id
}

output "security_group_rds_id" {
  description = "Security group ID for RDS"
  value       = aws_security_group.rds.id
}
